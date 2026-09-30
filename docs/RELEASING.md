# Releasing

## A new version

1. Set `version` in `Cargo.toml` (workspace) and the default `AppVersion` in `installer/dpimech.iss`,
   add a line to `%changelog` in `packaging/fedora/dpimech.spec`, and add a work-log entry to `docs/PROGRESS.md`.
2. Commit and push to `main`; wait for CI to pass.
3. Push `main` to the `release` branch; the workflow creates the tag `v<version>` itself:

   ```sh
   git push origin main:release
   ```

   (Pushing a tag `v<version>` by hand works too; it must match `Cargo.toml`. A version that is
   already released is refused: raise it first.)

4. The `release` workflow builds the Windows installer, the Linux `.deb` / `.rpm` / AppImage and the
   macOS `.dmg`, publishes them with `SHA256SUMS.txt` as a GitHub release, and then starts the Fedora
   COPR build (below).

A dry run without publishing: push the commit to the `release-check` branch.

## Fedora COPR (one-time setup)

Fedora users install from the COPR repository `halilkahraman/DPIMech`:
`sudo dnf copr enable halilkahraman/DPIMech && sudo dnf install dpimech`.
COPR builds the RPM from source: `.copr/Makefile` → `packaging/fedora/make-srpm.sh` makes a source
RPM from the **newest `v*` tag** (all crates vendored), and COPR builds it without network.

1. On <https://copr.fedorainfracloud.org>, **New project**:
   - Project name: `DPIMech` (the existing project; COPR project names are case-sensitive and cannot be renamed)
   - Chroots: the supported Fedora releases, `x86_64` and `aarch64` (e.g. `fedora-42`, `fedora-43`).
     Fedora must ship Rust 1.85 or newer, which all current releases do.
   - Other settings can stay as they are (internet access during the build is **not** needed).
2. In the project, **Packages → New package**, source type **SCM**:
   - Package name: `dpimech`
   - Clone URL: `https://github.com/halilkhrmn/dpimech.git`
   - Committish and subdirectory: empty
   - Spec file: `packaging/fedora/dpimech.spec`
   - SRPM build method: **make_srpm**
   - Leave "Auto-rebuild" off: builds should follow releases, not every push to `main`.
3. Press **Rebuild** on the package once to check that it builds.
4. **Settings → Integrations** lists several webhooks; take the first one under **Custom webhook(s)**
   and replace `<PACKAGE_NAME>` with the package name: `…/webhooks/custom/<id>/<secret>/dpimech/`
   (not the GitHub one: it expects GitHub's payload) and save it on GitHub:
   **Settings → Secrets and variables → Actions → New repository secret**,
   name `COPR_WEBHOOK_URL`. From then on every published release starts a COPR build.
   (If a POST to the URL does not start a build, turn on "Auto-rebuild" for the package.)
5. Test it: **Actions → COPR build → Run workflow**. The log says what COPR answered, or what is
   wrong with the secret (placeholder left in, not a COPR URL, …). The same workflow starts a COPR
   build by hand at any time; `tools/copr-webhook.sh` is shared with the release workflow.

Build locally: `sh packaging/fedora/make-srpm.sh target/srpm` (needs `cargo`, `git`, `rpm-build`),
then `rpmbuild --rebuild target/srpm/dpimech-*.src.rpm` on Fedora, or `mock` for a clean chroot.
