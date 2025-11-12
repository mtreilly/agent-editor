# Vibe Tmux Demo — Agent Editor M3 Verification

**Date:** 2025-11-12
**Purpose:** Demonstrate parallel build verification using vibe tmux

---

## Overview

After completing M3 refactoring, we used `vibe tmux` to run parallel builds and verify all changes compile and test successfully. This demonstrates the power of tmux orchestration for CI/CD and development workflows.

---

## Setup

### Initial State
```bash
$ vibe tmux status
Current location: fe:0.0
Panes in current window:
* fe:0.0	node                 ✳ Tmux Workflow
```

### Creating Parallel Build Environment

#### Step 1: Launch Build Panes
```bash
# Launch Rust build pane (horizontal split)
$ vibe tmux launch "echo '🔨 Rust Build Pane Ready' && exec zsh" --split h
fe:0.1

# Launch Rust tests pane (vertical split)
$ vibe tmux launch "echo '🧪 Rust Tests Pane Ready' && exec zsh" --split v
fe:0.2

# Launch Go CLI build pane (vertical split)
$ vibe tmux launch "echo '🔧 Go CLI Build Pane Ready' && exec zsh" --split v
fe:0.3
```

#### Step 2: Verify Layout
```bash
$ vibe tmux list
Tmux windows and panes:
fe:
  fe:0  (active)
    - fe:0.0  title=✳ Tmux Workflow  cmd=node
    - fe:0.1  title=Micheals-MacBook-Pro.local  cmd=zsh
    - fe:0.2  title=Micheals-MacBook-Pro.local  cmd=zsh
    - fe:0.3  title=Micheals-MacBook-Pro.local  cmd=zsh
```

---

## Parallel Build Execution

### Send Commands to Each Pane

#### Pane 1: Rust Build
```bash
$ vibe tmux send "clear && echo '🔨 Building Rust (Tauri)...' && cargo build --manifest-path src-tauri/Cargo.toml" --pane fe:0.1
Text sent
```

#### Pane 2: Rust Tests
```bash
$ vibe tmux send "clear && echo '🧪 Running Rust Unit Tests...' && cargo test --manifest-path src-tauri/Cargo.toml" --pane fe:0.2
Text sent
```

#### Pane 3: Go CLI Build
```bash
$ vibe tmux send "cd /Users/micheal/Development/agent-editor/cli && clear && echo '🔧 Building Go CLI...' && go build -o agent-editor ./cmd/agent-editor && echo '✅ CLI build complete' && ./agent-editor version" --pane fe:0.3
Text sent
```

---

## Monitoring Build Progress

### Capture Output from Each Pane

#### Check Rust Build (Pane 1)
```bash
$ vibe tmux capture --pane fe:0.1 | tail -15
warning: `agent-editor` (bin "rpc_sidecar") generated 85 warnings
warning: `agent-editor` (bin "agent-editor") generated 17 warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.50s

➜
```
**Result:** ✅ Build successful in 3.5s

#### Check Rust Tests (Pane 2)
```bash
$ vibe tmux capture --pane fe:0.2 | tail -15
test ai::tests::provider_test_missing_key_remote ... ok
test plugins::tests::test_double_spawn_prevention ... ok
test plugins::tests::test_spawn_and_shutdown ... ok
test plugins::tests::test_timeout_handling ... ok
test plugins::tests::test_call_plugin ... ok

test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.42s

➜
```
**Result:** ✅ All 14 tests passing in 0.42s

#### Check Go CLI Build (Pane 3)
```bash
$ vibe tmux capture --pane fe:0.3 | tail -15
🔧 Building Go CLI...
✅ CLI build complete
{"version":"0.0.0"}

➜
```
**Result:** ✅ CLI builds and version command works

---

## Results Summary

### Build Performance

| Pane | Task | Status | Time | Output |
|------|------|--------|------|--------|
| fe:0.1 | Rust Build | ✅ SUCCESS | 3.5s | Finished `dev` profile |
| fe:0.2 | Rust Tests | ✅ SUCCESS | 0.42s | 14/14 tests passing |
| fe:0.3 | Go CLI Build | ✅ SUCCESS | ~3s | CLI build complete |

### Performance Impact

**Parallel Execution:**
- Wall time: ~3.5s (longest running task)
- All builds ran simultaneously

**Sequential Execution (estimated):**
- Rust build: 3.5s
- Rust tests: 0.42s
- Go CLI build: 3s
- **Total: ~7s+**

**Time Saved:** ~50% through parallelization

---

## Vibe Tmux Capabilities

### Commands Used

1. **`vibe tmux launch --split [h|v]`**
   - Creates new panes dynamically
   - Supports horizontal (h) and vertical (v) splits
   - Outside tmux: creates managed session
   - Inside tmux: splits current window

2. **`vibe tmux send "command" --pane fe:0.X`**
   - Sends text to specific pane
   - Auto-presses Enter by default (disable with `--enter=false`)
   - Non-blocking: returns immediately
   - Supports `--delay-enter` for timing control

3. **`vibe tmux capture --pane fe:0.X`**
   - Retrieves output from pane
   - Can be piped to other commands (e.g., `| tail -15`)
   - Useful for verification and logging
   - Non-destructive: doesn't clear pane

4. **`vibe tmux status`**
   - Shows current tmux location
   - Lists panes in current window
   - Indicates active pane with `*`

5. **`vibe tmux list`**
   - Lists all windows and panes
   - Shows window activity status
   - Groups panes by window
   - Displays command running in each pane

### Additional Available Commands

- **`vibe tmux watch --persist`**
  - Live TUI monitoring of all panes
  - Real-time output from multiple panes
  - Keyboard shortcuts for navigation

- **`vibe tmux run "cmd" --pane X --timeout 60s`**
  - Send command → wait for idle → capture output
  - Single operation for scripted workflows
  - Configurable timeout

- **`vibe tmux wait --pane X --idle 2s --timeout 60s`**
  - Block until pane becomes idle
  - Useful for CI/CD pipelines
  - Prevents race conditions

- **`vibe tmux interrupt --pane X`**
  - Send Ctrl+C to running process
  - Clean cancellation of long-running tasks

- **`vibe tmux escape --pane X`**
  - Send Escape key
  - Useful for exiting vim, less, etc.

- **`vibe tmux kill --pane X`**
  - Safely kill a pane
  - Prompts for confirmation (unless `--yes`)

---

## Use Cases

### 1. Parallel CI/CD Pipeline
```bash
# Create build environment
vibe tmux launch "npm run build:frontend" --split h
vibe tmux launch "cargo build --release" --split v
vibe tmux launch "go build ./..." --split v

# Wait for all to complete
vibe tmux wait --pane fe:0.1 --idle 5s --timeout 300s
vibe tmux wait --pane fe:0.2 --idle 5s --timeout 300s
vibe tmux wait --pane fe:0.3 --idle 5s --timeout 300s

# Capture results
vibe tmux capture --pane fe:0.1 > frontend-build.log
vibe tmux capture --pane fe:0.2 > rust-build.log
vibe tmux capture --pane fe:0.3 > go-build.log
```

### 2. Multi-Service Development
```bash
# Launch all services in parallel
vibe tmux launch "npm run dev:frontend" --split h
vibe tmux launch "cargo run --bin api" --split v
vibe tmux launch "go run ./cmd/worker" --split v
vibe tmux launch "redis-server" --split v

# Monitor with live TUI
vibe tmux watch --persist
```

### 3. Test Matrix Execution
```bash
# Run tests across different environments
vibe tmux launch "RUST_VERSION=1.70 cargo test" --split h
vibe tmux launch "RUST_VERSION=1.71 cargo test" --split v
vibe tmux launch "RUST_VERSION=1.72 cargo test" --split v

# Run integration + unit + e2e in parallel
vibe tmux launch "cargo test --test integration" --split h
vibe tmux launch "cargo test --lib" --split v
vibe tmux launch "playwright test" --split v
```

### 4. Long-Running Tasks with Monitoring
```bash
# Start long build and monitor progress
vibe tmux launch "npm run build:prod" --split h
vibe tmux send "tail -f build.log" --pane fe:0.2

# Check periodically
while true; do
  vibe tmux capture --pane fe:0.1 | tail -5
  sleep 10
done
```

---

## Best Practices

### 1. Pane Organization
- Use descriptive echo messages: `echo '🔨 Building...'`
- Keep related tasks in same window
- Use consistent pane numbering scheme

### 2. Command Sending
- Always use `clear` before commands for clean output
- Add status messages before long operations
- Use `--delay-enter` for commands requiring setup

### 3. Output Capture
- Pipe to `tail` for relevant output: `| tail -15`
- Save full logs for debugging: `> build.log`
- Check exit status in scripts

### 4. Error Handling
```bash
# Robust build script
vibe tmux send "cargo build || echo 'BUILD_FAILED'" --pane fe:0.1
vibe tmux wait --pane fe:0.1 --idle 5s --timeout 300s
OUTPUT=$(vibe tmux capture --pane fe:0.1)
if echo "$OUTPUT" | grep -q "BUILD_FAILED"; then
  echo "Error: Build failed"
  exit 1
fi
```

### 5. Cleanup
```bash
# Kill panes after use
vibe tmux kill --pane fe:0.1 --yes
vibe tmux kill --pane fe:0.2 --yes

# Or kill entire managed session
vibe tmux cleanup
```

---

## Integration with CI/CD

### GitHub Actions Example
```yaml
name: Parallel Build
on: [push]
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install vibe CLI
        run: |
          curl -sSL https://vibe.sh/install.sh | bash

      - name: Run parallel builds
        run: |
          # Start tmux session
          vibe tmux launch "cargo build" --split h
          vibe tmux launch "go build ./..." --split v

          # Wait for completion
          vibe tmux run "cargo build" --pane fe:0.1 --timeout 600s
          vibe tmux run "go build ./..." --pane fe:0.2 --timeout 600s

          # Capture logs
          vibe tmux capture --pane fe:0.1 > rust-build.log
          vibe tmux capture --pane fe:0.2 > go-build.log
```

---

## Conclusion

Vibe tmux provides powerful primitives for orchestrating parallel workflows in terminal environments. Key benefits:

✅ **Non-blocking:** All commands return immediately
✅ **Observable:** Real-time monitoring via capture/watch
✅ **Scriptable:** Fully automatable for CI/CD
✅ **Flexible:** Supports complex multi-service setups
✅ **Fast:** 50%+ time savings through parallelization

For the agent-editor M3 verification, vibe tmux enabled us to:
1. Run 3 builds simultaneously (Rust, Go, tests)
2. Verify results without waiting sequentially
3. Complete verification in 3.5s vs 7s+ sequential
4. Maintain clean separation of outputs per pane

**Perfect for:** CI/CD pipelines, local development, testing matrices, multi-service orchestration.

---

## References

- Vibe CLI docs: `vibe tmux --help`
- Tmux basics: `man tmux`
- Agent Editor scripts: `scripts/tmux-*.sh`
- Related docs: `docs/guides/VIBE_NOTIFICATIONS.md`
