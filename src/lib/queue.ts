const MAX_ACTIVE = 3;
let active = 0;
const waiting: Array<() => void> = [];

export async function limited<T>(task: () => Promise<T>): Promise<T> {
  if (active >= MAX_ACTIVE) {
    await new Promise<void>((resolve) => waiting.push(resolve));
  }
  active += 1;
  try {
    return await task();
  } finally {
    active -= 1;
    waiting.shift()?.();
  }
}
