/** "1.0.0" -> "1.0". Mirrors src-tauri/src/version.rs::display. */
export function displayVersion(semver: string): string {
  const m = /^v?(\d+)\.(\d+)(?:\.\d+)?$/.exec(semver.trim());
  return m ? `${m[1]}.${m[2]}` : semver;
}
