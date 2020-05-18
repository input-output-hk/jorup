- Installation
  - **Cleanup**: `rm -r ~/.jorup`.
  - **Install**: `cargo install --path .` or go to the site and follow the
    instructions (for releases).
    - If installed from the website: check if `~/.jorup/bin` is in `PATH`.
- Blockchain configs:
  - **Update/install configs**: `jorup blockchain update`.
  - **Check configs**: `jorup blockchain list`. Should list nightly and ITN.
- Start without installed nodes: `jorup run itn [-v nightly | -v 0.8.19]`.
  - jorup should output `Cannot run without compatible release`.
  - `~/.jorup/release` should be empty or non existent.
  - `jorup node list` should be empty.
- Install the latest & nightly release: `jorup node install [-v nightly]`.
  - `jorup node list` should show the installed versions.
- `jorup run itn -v nightly` should run the latest nightly version. The
  following should work: 
  - `jorup info itn`
  - `jorup shutdowb itn`
  - The same should apply for `jorup run itn -v nightly -d`
