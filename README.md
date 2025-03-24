# Noita'd
A noita mod manager.

![](https://github.com/user-attachments/assets/1e6b3420-0235-438b-9eca-cba96af3a2d8)

## Building the Project

Before building the project, ensure that you have the following dependencies installed: `meson`, `flatpak`, and `flatpak-builder`.

### Setup Configuration

First, set up the project configuration using Meson:

```shell
meson setup build
```

### Build and Run Options

You have two options for building and running the project:

1. **Install and Run via Meson**:
    ```shell
    meson -C build install
    noitad
    ```

2. **Build and Run via Flatpak**:
    ```shell
    flatpak install --user \
        org.gnome.Sdk//47 \
        org.gnome.Platform//47 \
        org.freedesktop.Sdk.Extension.rust-stable//24.08 \
        org.freedesktop.Sdk.Extension.llvm18//24.08

    flatpak-builder --user build \
        build-aux/com.github.nozwock.noitad.Devel.json

    flatpak-builder --run build \
        build-aux/com.github.nozwock.noitad.Devel.json \
        noitad
    ```

Choose the method that best suits your workflow and environment.
