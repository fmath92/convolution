# Convolution WASM Explorer

Small `egui` app (native + WASM) to:

- Load a grayscale-convertible histological slide PNG (`lame histologique`).
- Load a PNG containing packed convolution kernels.
- Choose kernel shape (`3x6` or `6x3`).
- Split the kernel sheet into individual kernels.
- Run convolution for each kernel and visualize per-kernel result previews.

## Run natively

```bash
cargo run
```

## Run in browser (WASM)

Prerequisites:

- `rustup target add wasm32-unknown-unknown`
- [`trunk`](https://trunkrs.dev/)

Then:

```bash
trunk serve
```

Open the local URL printed by Trunk.

## Usage flow

1. Drag and drop two PNG files into the app window:
   - first: histological slide
   - second: packed kernels sheet
2. Select kernel shape (`3x6` or `6x3`).
3. Click `Split kernels`.
4. Click `Run all convolutions`.
5. Use the kernel index slider to visualize each result preview.
