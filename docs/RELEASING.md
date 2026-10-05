# Releasing

Pushing a `v*` tag publishes a GitHub Release: `.github/workflows/release.yml` builds every platform, checks the builds, and uploads the packages with a `SHA256SUMS` file and generated release notes. The web page and the relay image do not follow tags: they deploy from `main` (`web.yml`, `relay.yml`).

## Assets

| File | Built by | Contents |
|---|---|---|
| `SuperMarioWar-<tag>-linux-x86_64.tar.gz`, `-linux-aarch64.tar.gz` | `build_linux.yml` | `smw`, `leveleditor`, `worldeditor`, `smw_server`, `data/`, README, CREDITS; links the system SDL2 |
| `SuperMarioWar-<tag>-macos-universal.zip` | `build_macos.yml` | `Super Mario War.app`, arm64 and x86_64, dylibs bundled, ad-hoc signed |
| `SuperMarioWar-<tag>-windows-x86_64.zip` | `build_windows.yml` | the four `.exe` files, SDL2 DLLs and licenses, `data/` |
| `SuperMarioWar-<tag>-web.zip` | `build_web.yml` | the `tools/package_web.sh` bundle for any static web server |
| `SuperMarioWar-<tag>.muxapp` | `build_muxapp.yml` | the muOS package (`device/muos/README.md`) |
| `SuperMarioWar-<tag>.apk` | `build_android.yml` | the debug-signed Android APK (`android/README.md`) |
| `SHA256SUMS` | `release.yml` | checksums of the files above |

## Cutting a release

1. Check that CI is green on the `main` commit you will tag.
2. Optional: run a dry run from Actions > Release > Run workflow, or `gh workflow run release.yml --ref main`. It builds everything and uploads the packages as the `release-assets` artifact without creating a release. The run summary lists each file's size and checksum.
3. Set `version` in `Cargo.toml` to the new version (it becomes the macOS app's `CFBundleShortVersionString`), run `cargo check` to update `Cargo.lock`, and merge that change to `main`.
4. Tag the merge commit and push the tag:

   ```sh
   git switch main && git pull
   git tag -a v0.2.0 -m "v0.2.0"
   git push origin v0.2.0
   ```

5. Watch the Release run. When it finishes, the release is at `https://github.com/KyleJamesWalker/supermariowar-rust/releases/tag/v0.2.0`. Edit the generated notes if needed.

If a build fails, fix it on `main` and tag a new patch version. Re-running the workflow for an existing tag replaces its assets (`gh release upload --clobber`).

## Checking a download

```sh
sha256sum -c SHA256SUMS --ignore-missing
```
