// A tiny in-app toast bus, so any component can confirm an action (e.g. "Fn reset…") without
// threading a callback through the tree. App subscribes and renders the toast.
type Listener = (message: string) => void;

const listeners = new Set<Listener>();

export function subscribeToast(cb: Listener): () => void {
  listeners.add(cb);
  return () => listeners.delete(cb);
}

export function showToast(message: string): void {
  for (const cb of [...listeners]) {
    try {
      cb(message);
    } catch (e) {
      console.error("toast listener failed", e);
    }
  }
}
