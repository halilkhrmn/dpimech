//! The server name (SNI) of a TLS ClientHello, so the detailed log can say which site a
//! redirected connection was for instead of only an IP address.

/// `None` for anything that is not the start of a TLS ClientHello carrying a host name.
pub fn server_name(data: &[u8]) -> Option<String> {
    // Record header: handshake (0x16), version, length.
    if *data.first()? != 0x16 {
        return None;
    }
    let body = data.get(5..)?;
    // Handshake header: ClientHello (1), 3-byte length; then version (2) and random (32).
    if *body.first()? != 1 {
        return None;
    }
    let mut pos = 4 + 2 + 32;
    let skip = |pos: &mut usize, len_bytes: usize| -> Option<()> {
        let len = match len_bytes {
            1 => usize::from(*body.get(*pos)?),
            _ => usize::from(u16::from_be_bytes([*body.get(*pos)?, *body.get(*pos + 1)?])),
        };
        *pos += len_bytes + len;
        Some(())
    };
    skip(&mut pos, 1)?; // session id
    skip(&mut pos, 2)?; // cipher suites
    skip(&mut pos, 1)?; // compression methods
    let ext_len = usize::from(u16::from_be_bytes([*body.get(pos)?, *body.get(pos + 1)?]));
    pos += 2;
    let end = (pos + ext_len).min(body.len());
    while pos + 4 <= end {
        let kind = u16::from_be_bytes([body[pos], body[pos + 1]]);
        let len = usize::from(u16::from_be_bytes([body[pos + 2], body[pos + 3]]));
        pos += 4;
        if kind == 0 {
            // server_name list: list length (2), type (1) = host_name, name length (2), name.
            let ext = body.get(pos..pos + len)?;
            if *ext.get(2)? != 0 {
                return None;
            }
            let name_len = usize::from(u16::from_be_bytes([*ext.get(3)?, *ext.get(4)?]));
            let name = ext.get(5..5 + name_len)?;
            return std::str::from_utf8(name)
                .ok()
                .filter(|n| n.bytes().all(|b| b.is_ascii_graphic()))
                .map(str::to_owned);
        }
        pos += len;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::server_name;

    /// A minimal ClientHello for `host` with one extra extension before the SNI.
    fn client_hello(host: &str) -> Vec<u8> {
        let name = host.as_bytes();
        let mut sni = Vec::new();
        sni.extend_from_slice(&((name.len() + 3) as u16).to_be_bytes());
        sni.push(0);
        sni.extend_from_slice(&(name.len() as u16).to_be_bytes());
        sni.extend_from_slice(name);
        let mut exts = vec![0x00, 0x17, 0x00, 0x00]; // extended_master_secret, empty
        exts.extend_from_slice(&[0x00, 0x00]);
        exts.extend_from_slice(&(sni.len() as u16).to_be_bytes());
        exts.extend_from_slice(&sni);
        let mut hello = vec![0x03, 0x03];
        hello.extend_from_slice(&[7; 32]);
        hello.push(0); // session id
        hello.extend_from_slice(&[0x00, 0x02, 0x13, 0x01]); // one cipher suite
        hello.extend_from_slice(&[0x01, 0x00]); // compression: null
        hello.extend_from_slice(&(exts.len() as u16).to_be_bytes());
        hello.extend_from_slice(&exts);
        let mut handshake = vec![1, 0];
        handshake.extend_from_slice(&(hello.len() as u16).to_be_bytes());
        handshake.extend_from_slice(&hello);
        let mut record = vec![0x16, 0x03, 0x01];
        record.extend_from_slice(&(handshake.len() as u16).to_be_bytes());
        record.extend_from_slice(&handshake);
        record
    }

    #[test]
    fn finds_the_host_name() {
        assert_eq!(
            server_name(&client_hello("gateway.discord.gg")).as_deref(),
            Some("gateway.discord.gg")
        );
    }

    #[test]
    fn ignores_other_data_and_truncated_hellos() {
        assert_eq!(server_name(b"GET / HTTP/1.1\r\n"), None);
        let hello = client_hello("discord.com");
        assert_eq!(server_name(&hello[..20]), None);
        assert_eq!(server_name(&[]), None);
    }
}
