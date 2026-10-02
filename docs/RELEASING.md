# Releasing

## A new version

`main` only changes through pull requests. A release is a pull request that raises the version:

1. On a branch: set `version` in `Cargo.toml` (workspace) and the default `AppVersion` in
   `installer/dpimech.iss`, add a line to `%changelog` in `packaging/fedora/dpimech.spec`, write the
   user-facing notes as a `## <version>` section in `changelog/en.md` and its translations
   (`tr.md`, `ru.md`, `fa.md`, `ar.md`; the app shows them once after the update as "What's new", the
   GitHub release uses the English one, and `cargo test` fails while a language lacks the section),
   and add a work-log entry to `docs/PROGRESS.md`.
2. Open the pull request; merge it when CI is green.
3. On the merge, the `Release` workflow sees a version without a tag and publishes it: it builds the
   Windows installer, the Linux `.deb` / `.rpm` / AppImage and the macOS `.dmg`, creates the tag
   `v<version>` with a GitHub release (files + `SHA256SUMS.txt`), starts the Fedora COPR build and
   attaches the Arch package (both below). Merges that keep the version publish nothing.

Before anything is published, **smoke tests** install each package on a clean system and start it
(`tools/smoke-linux.sh`: AppImage on Arch, `.deb` on Ubuntu, `.rpm` on Fedora; `tools/smoke-windows.ps1`:
silent install, service, window, uninstall). If one fails, nothing is published: the job's log says which
library is missing or how the app failed.

Pushing a tag `v<version>` by hand also publishes (it must match `Cargo.toml`).
A dry run without publishing: **Actions → Release → Run workflow**.

The landing page (GitHub Pages) needs no change per release: it reads the newest release from the
GitHub API and offers the file for the visitor's system.

## Protecting `main` (one-time)

**Settings → Rules → Rulesets → New ruleset → New branch ruleset**: name `main`, enforcement
**Active**, target **Include default branch**, and turn on **Restrict deletions**, **Require a pull
request before merging** (0 approvals is fine for a one-person project) and **Block force pushes**.
Optionally **Require status checks to pass** with the CI jobs.

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

## Arch package (AUR on hold)

Arch, CachyOS, EndeavourOS and Manjaro users install `dpimech-bin-<version>-1-x86_64.pkg.tar.zst` from the
release with `sudo pacman -U`. It repackages the release `.deb` (`packaging/aur/`); the release workflow builds
it with `tools/aur-publish.sh`, installs it in a clean Arch and attaches it to the release. No setup needed.

Publishing to the AUR (`yay -S dpimech-bin`) is on hold because the AUR takes no new accounts (DECISIONS #48).
When it does again: create the account, add an SSH key to it, store the private key as the repository secret
`AUR_SSH_PRIVATE_KEY`, and pass `--push` to `tools/aur-publish.sh` in release.yml (the push code is still there).

## Changing the site lists

`strategies/domains.json` holds the sites the wizard, the Lab and the profile editor offer, fetched from `main`
the same way as the strategies below (no version bump needed). `probes` must answer HTTPS on `/` without a bot
check (a 403 counts as blocked); `countries` lists ISO codes of the countries where the site is widely reported
blocked (`*` = offered first everywhere): users there get these first and preselected; `names` translates a
pack name that is not a brand. `cargo test` checks the file.

## Changing the standard strategies

`strategies/default.json` holds the Strategy Lab's standard set per engine (`zapret_winws` is also used for
nfqws on Linux). Change it in a pull request; once it is merged, running services pick it up within a day
(daily update check) or at once with "Update online lists" in the Lab — no version bump needed. Keep `"format": 1`:
a build ignores a file with a format it does not know. `cargo test` checks that the file parses and that
ByeDPI, tpws and SpoofDPI strategies pass the argument policy; the policy checks every strategy again at launch.
