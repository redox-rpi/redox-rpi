# Experiment: orbital-apps, a few desktop apps for Orbital (#33)

On top of experiment [`orbital`](../orbital/README.md): a browser, a text
editor and a file manager from upstream's `config/desktop.toml`, taken as
binary aarch64 packages from `static.redox-os.org` like the rest of the
image.

```bash
scripts/build-image.sh --experiment timesync --experiment rpi-fb \
    --experiment rpi-fb-dma-on --experiment orbital --experiment usb-mouse \
    --experiment orbital-apps
```

- [`config.toml`](config.toml): `netsurf`, `cosmic-edit`, `cosmic-files`,
  `cosmic-icons`, `hicolor-icon-theme`. Each app puts its own entry in
  `/usr/share/ui/apps/`, so the launcher shows it. `cosmic-term` is left
  out; `orbterm` stays the terminal.

## Sizes

Installed size (`storage_size` in the package index, 2026-11-02), with the
dependencies the image does not have yet:

| app | package | with new dependencies | biggest dependencies |
|---|---|---|---|
| NetSurf 3.11 (browser) | 3.8 MB | 57.4 MB | openssl3 24.5, liborbital 9.7, libjpeg 4.3, curl 3.5 |
| COSMIC Text Editor 1.10 | 27.1 MB | 48.5 MB | libxml2 7.2, xkeyboard-config 7.0, libxcb 5.5 |
| COSMIC Files 1.11 | 28.2 MB | 28.2 MB more | shares the editor's |
| cosmic-icons, hicolor-icon-theme | 0.9 MB | 0.9 MB | |
| **all** | | **135 MB** | |

Much of it is static libraries and headers in the library packages (for
example openssl3), which the binary packages carry along.
