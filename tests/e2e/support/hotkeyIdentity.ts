export interface ForegroundIdentity {
  pid: number;
  executable: string | null;
  title: string | null;
  creationTime: string | null;
  error: number;
}

export function assertForegroundGameIdentity(
  expected: { pid: number; executable: string; title: string },
  observed: ForegroundIdentity | null | undefined,
  previous?: ForegroundIdentity,
): asserts observed is ForegroundIdentity {
  if (!observed || observed.error !== 0)
    throw new Error(`Foreground process identity unavailable: ${observed?.error ?? 'missing'}`);
  if (
    observed.pid !== expected.pid ||
    observed.executable?.toLowerCase() !== expected.executable.toLowerCase() ||
    observed.title !== expected.title ||
    !observed.creationTime ||
    !/^[0-9a-f]{16}$/i.test(observed.creationTime) ||
    observed.creationTime === '0000000000000000' ||
    (previous && previous.creationTime !== observed.creationTime)
  )
    throw new Error('Foreground executable, title or process birth identity mismatch');
}
