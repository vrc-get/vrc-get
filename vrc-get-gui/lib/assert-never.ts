
export function assertNever(x: never, message?: string): never {
  if (message) {
    throw new Error("Unexpected object: " + x + ": " + message);
  } else {
    throw new Error("Unexpected object: " + x);
  }
}
