/** One physical key. Units are key units (1u = Layout.unit mm); row 0 is the F row. */
export type LayoutKey = {
  code: string; // W3C KeyboardEvent.code, e.g. "KeyA", "NumpadEnter"
  row: number;
  x: number;
  w: number;
  h: number;
  mac: string; // legend printed on the Macally
  win: string; // what Windows calls the key
  verify?: boolean; // OS code not yet confirmed on hardware
};

export type Layout = { unit: number; keys: LayoutKey[] };
