#!/bin/sh
# Asks Fedora COPR to build the dpimech package (it builds the newest v* tag).
# The URL comes from $COPR_WEBHOOK_URL: COPR → Settings → Integrations → custom webhook,
# with the package name at the end. Used by the release and copr workflows.
set -eu
# Pasted secrets often carry a trailing newline or spaces.
url=$(printf '%s' "${COPR_WEBHOOK_URL:-}" | tr -d '[:space:]')
case "$url" in
    *'<'* | *'>'*)
        echo "::error::COPR_WEBHOOK_URL still contains a placeholder like <PACKAGE_NAME>: replace it with dpimech"
        exit 1 ;;
    https://copr.fedorainfracloud.org/webhooks/custom/*/*/dpimech/ | https://copr.fedorainfracloud.org/webhooks/custom/*/*/dpimech) ;;
    '')
        echo "::error::COPR_WEBHOOK_URL is not set"
        exit 1 ;;
    *)
        echo "::error::COPR_WEBHOOK_URL should look like https://copr.fedorainfracloud.org/webhooks/custom/<number>/<secret>/dpimech/ (got ${#url} characters starting with $(printf '%s' "$url" | cut -c1-45))"
        exit 1 ;;
esac
code=$(curl -sS -o /tmp/copr-reply -w '%{http_code}' -X POST "$url") || {
    echo "::error::could not reach COPR"
    exit 1
}
echo "COPR answered HTTP $code: $(head -c 300 /tmp/copr-reply)"
case "$code" in
    2*) ;;
    *) echo "::error::COPR refused the webhook (HTTP $code); check the URL, or press Rebuild in COPR"; exit 1 ;;
esac
