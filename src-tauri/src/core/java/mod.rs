//! `JavaManager`: finds Java runtimes on the machine (`PATH`, `JAVA_HOME`,
//! known install locations, the Windows registry, and TapkaCraft's own
//! managed `runtimes/` dir), verifies each one by actually running
//! `java -version` rather than trusting a folder/registry name, and picks
//! the runtime matching what a given Minecraft version requires. Never
//! touches the user's system Java installation, `JAVA_HOME`, or `PATH` -
//! only reads them.

pub mod detect;
pub mod runtime;
