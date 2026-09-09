//! `JavaManager` (Phase 3).
//!
//! Locates usable Java runtimes on Windows (registry `Software\JavaSoft`
//! and `Software\Eclipse Adoptium` keys, `PATH`, `JAVA_HOME`, and known
//! install locations), verifies each candidate by actually running
//! `java -version` rather than trusting folder names, and picks the
//! runtime matching a given Minecraft version's required Java major
//! version. Also manages TapkaCraft's own downloaded runtimes under the
//! app's local data dir (never the user's system Java/`JAVA_HOME`/`PATH`),
//! and exposes a per-instance manual override. Launches prefer
//! `javaw.exe` to avoid a console window; `java.exe` is used when console
//! output is explicitly wanted (debugging).
