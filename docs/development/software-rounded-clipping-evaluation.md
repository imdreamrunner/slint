# Software renderer rounded clipping evaluation

This fork evaluates the upstream fix for rounded child clipping on an embedded touchscreen.
It does not change Slint's public API.

## Defect and version scope

Slint 1.18.1 clips children of a rounded `Rectangle` to a rectangular area, even with `clip: true`.
An opaque child image can therefore cover the parent's rounded corners.
The software renderer documents this limitation and ignores the radius in `combine_clip()`.

Upstream commit [`11a7d7d6`](https://github.com/slint-ui/slint/commit/11a7d7d6d3196ee9d9dc3d84bab5eb7b5bc2669c) implements the missing support.
This draft preserves its authorship and records its source with a cherry-pick trailer.
The comparison base is its immediate parent, `afbcf51ee5c4d355e5f21d70440ee4ca2f5f3913`.
Both belong to Slint's 1.19 development tree.

This is an evaluation of the existing upstream fix, not a backport to 1.18.1.
Internal Slint crates use exact version pins and must move together.
The patch depends on clipping and gradient infrastructure introduced after 1.18.1.

## Implementation

Rounded clips use physical screen coordinates and follow the inner border edge.
Commands that reach a rounded corner carry a clip run.
Nested clips intersect without multiplying identical edge coverage.
Both `render()` and `render_by_line()` apply the run during blending.

Clipped commands bypass acceleration hooks that cannot honor rounded clipping.
The line renderer also stops treating them as guaranteed opaque.
Commands away from rounded corners retain their fast paths.
The implementation adds clip metadata rather than a full-screen mask or a second renderer.

## Independent regression tests

`tests/screenshots/rounded_clipping.rs` tests a runtime image under a rounded clip and an unclipped sibling.
It covers RGB888 and RGB565, four rotations, two scale factors, nested clips, borders, and both rendering paths.
The matrix has 128 configurations.

Each configuration checks that the image leaves the outer corner uncovered.
It changes the image and requires a nonempty redraw smaller than the screen.
The resulting pixels must equal a fresh full render exactly.
Two additional dirty regions cross rounded corners and must repair deliberately corrupted pixels.
An unclipped sibling verifies that clip state does not leak.

The negative control runs these tests with only the renderer reverted to the comparison base.
Both pixel formats fail at the corner assertion: the child image covers a pixel belonging to the window background.
The same tests pass with the upstream patch restored.

Run the focused screenshot and independent regression tests:

```sh
SLINT_TEST_FILTER=clip-border-radius cargo test --manifest-path tests/Cargo.toml \
    -p test-driver-screenshots --no-default-features --features software,software-embed-assets
```

Run broader checks:

```sh
cargo test --manifest-path tests/Cargo.toml -p test-driver-screenshots \
    --no-default-features --features software,software-embed-assets
cargo test -p i-slint-renderer-software --lib
cargo check -p i-slint-renderer-software --no-default-features \
    --features libm,i-slint-core/unsafe-single-threaded
```

The no-std configuration uses Slint's single-threaded core feature for this compilation check.
It is not a configuration recommendation for the multi-threaded Raspberry Pi application.

### Local results

Checked on November 1, 2026, on Linux x86-64 with Rust 1.99.1:

| Check | Result |
| --- | --- |
| Unpatched renderer with independent regressions | Both tests fail at the uncovered-corner assertion, as expected |
| Focused screenshot and independent regressions | 8 passed |
| Full software screenshot suite | 149 passed, 9 ignored, 0 failed |
| Software renderer unit tests | 44 passed, 0 failed |
| Configured no-std compilation check | Passed |
| New Rust test formatting and diff whitespace | Passed |

The ignored screenshot tests are existing exclusions in the upstream suite.
No reference images were regenerated to make the tests pass.
Skia, GPU renderers, QEMU, and Raspberry Pi hardware were not tested with this fork.

## Raspberry Pi integration

The Pi application remains on Slint 1.18.1 with rounded native cards and inset transparent artwork.
Its flashed image does not contain this fork.
Keep that layout until a coordinated Slint upgrade passes the application's existing tests and ARM rendering checks.

Before deployment, measure rendering time, memory, changed framebuffer rows, and touch feedback on the Pi.
Compare identical screens with the current version and the candidate version.
These host tests establish correctness; they do not establish Raspberry Pi performance or SPI transfer speed.
