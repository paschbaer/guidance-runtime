# Applying this update

Copy these three files into the root of the Guidance Runtime repository, replacing the existing files:

- `.github/workflows/release.yml`
- `README.md`
- `README.de.md`

Then commit and push the change to `develop`. Because the release workflow itself is included in the path filters, that push creates a beta release unless the commit includes `[skip release]`.
