// Keys the keyboard's firmware handles and never reports to the OS, so no app can see them or use
// them as a remap source. On the Macally this is only Fn (keymap::FIRMWARE_ONLY in Rust; confirmed
// by a HID probe: the keyboard exposes standard keyboard, consumer and system-control collections
// and no vendor collection, and Fn appears on none of them).
const FIRMWARE_ONLY = new Set(["Fn"]);

export const isFirmwareOnly = (code: string) => FIRMWARE_ONLY.has(code);
