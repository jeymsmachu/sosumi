# Performance

Sosumi targets low resource use, which shapes several decisions. This documents
what that means concretely and how to check it.

## The budget

| Metric | Target | Why |
|---|---|---|
| Idle memory | < 150 MB | A music player should not cost more than a browser tab |
| Idle CPU | ~0% | Nothing polls when nothing is playing |
| Playback CPU | < 3% of one core for AAC, < 6% for ALAC | Decoding is not a background job |
| Startup to first frame | < 1.5 s | The window appears before the library loads |

## What makes it possible

**One process.** No sidecar engine, no HTTP surface, no IPC serialisation. See
`adr/0001-single-process.md`.

**Pure-Rust decode.** Symphonia decodes in-process with no C media framework and
no bundled runtime. See `adr/0006-native-audio-stack.md`.

**Event-driven, not polled.** The core publishes on change. The UI never polls
for position or state; the only periodic work is one position event per 250 ms
during playback.

**Work is owned by the thread that needs it.** The audio worker holds the device
and the decoder outright, so the hot path takes no locks. Bytes cross the
boundary once, on load.

**Fetches are sequential for a reason.** HLS segments are byte ranges of one
file. Fetching them in parallel would need reordering buffers to reconstruct the
stream, which costs more memory than the latency it saves.

## What to watch

Three things will show up first if something regresses:

1. **Idle CPU above zero.** Usually a poll that should be an event, or a worker
   that spins instead of sleeping. The audio worker deliberately sleeps when the
   output buffer is healthy.
2. **Memory growth during playback.** The player holds one decoded track. Growth
   across tracks means a leak in the handoff, not the decoder.
3. **Frame drops on scroll.** Twenty thousand list rows must not each build an
   image widget; artwork is fetched lazily per visible row.

## Measuring

```bash
# Idle footprint, after the window is up and nothing is playing
ps -o rss=,pcpu= -C sosumi

# Playback cost, while a track plays
top -b -n 5 -p "$(pgrep -x sosumi)" | tail -5

# Allocation profile (needs a nightly toolchain)
cargo build --release
valgrind --tool=massif ./build/linux/x64/release/bundle/sosumi
```

The Rust side is measured with `cargo bench` where a benchmark exists. The
Flutter side is measured with the DevTools performance view, which is the only
way to see the frame budget.
