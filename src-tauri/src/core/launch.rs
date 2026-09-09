//! `LaunchEngine` (Phase 3).
//!
//! Turns a resolved instance + authenticated account into a running
//! Minecraft process: verifies the instance's files, the selected Java
//! runtime and the account's session are all in order, prepares natives,
//! and builds the classpath, JVM arguments and game arguments as an
//! argument list (never hand-built string concatenation) before spawning
//! `java`/`javaw`. Streams stdout/stderr back to the UI as a live log and
//! reports the exit code so `CrashDoctor` (Phase 8) has something to
//! analyze on a non-zero exit.
