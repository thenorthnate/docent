// Browser fixtures: a small multi-file PR with moves, and generated large files.
import { lineDiff } from './diff';
import { detectMoves } from './moves';
import type { Annotation, FileDiff, Fixture, Hunk } from './model';

// ---------------------------------------------------------------------------
// Moves demo: within-file move, cross-file move, cross-file move with changes.

const parseToken = `export function parseToken(raw: string): Token {
  const [header, payload, signature] = raw.split('.');
  if (!header || !payload || !signature) {
    throw new TokenError('malformed token');
  }
  const claims = JSON.parse(atob(payload)) as Claims;
  return { header, claims, signature };
}
`;

const refreshSession = `export async function refreshSession(s: Session): Promise<Session> {
  const res = await fetch('/auth/refresh', {
    method: 'POST',
    headers: { authorization: \`Bearer \${s.refreshToken}\` },
  });
  if (!res.ok) throw new SessionError(res.status);
  const body = await res.json();
  return { ...s, token: parseToken(body.token), expiresAt: body.expiresAt };
}
`;

const validateClaimsOld = `export function validateClaims(claims: Claims, now: number): void {
  if (claims.exp <= now) throw new TokenError('expired');
  if (claims.nbf && claims.nbf > now) throw new TokenError('not yet valid');
  if (!claims.sub) throw new TokenError('missing subject');
  if (claims.aud !== AUDIENCE) throw new TokenError('wrong audience');
}
`;

const validateClaimsNew = `export function validateClaims(claims: Claims, now: number): void {
  if (claims.exp <= now) throw new TokenError('expired');
  if (claims.nbf !== undefined && claims.nbf > now) throw new TokenError('not yet valid');
  if (!claims.sub) throw new TokenError('missing subject');
  if (claims.aud !== AUDIENCE) throw new TokenError('wrong audience');
}
`;

const sessionHeader = `import { Session, SessionError } from './types';
import { parseToken } from './tokens';

const SKEW_MS = 30_000;
`;

const isExpired = `export function isExpired(s: Session, now = Date.now()): boolean {
  return s.expiresAt - SKEW_MS <= now;
}
`;

const signOut = `export function signOut(s: Session): void {
  localStorage.removeItem('session');
  s.listeners.forEach((l) => l('signed-out'));
}
`;

function demoFiles(): FileDiff[] {
  const raw = [
    {
      path: 'src/auth/session.ts',
      // refreshSession moves to the end of the file; parseToken moves to tokens.ts.
      original: [sessionHeader, parseToken, refreshSession, isExpired, signOut].join('\n'),
      modified: [sessionHeader, isExpired, signOut, refreshSession].join('\n'),
    },
    {
      path: 'src/auth/tokens.ts',
      original: `import { Token, TokenError, Claims } from './types';\n\nexport const AUDIENCE = 'docent';\n`,
      modified: [`import { Token, TokenError, Claims } from './types';\n\nexport const AUDIENCE = 'docent';\n`, parseToken, validateClaimsNew].join('\n'),
    },
    {
      path: 'src/auth/claims.ts',
      original: [`import { Claims, TokenError } from './types';\nimport { AUDIENCE } from './tokens';\n`, validateClaimsOld, `export const CLOCK_SKEW = 30;\n`].join('\n'),
      modified: [`import { Claims, TokenError } from './types';\nimport { AUDIENCE } from './tokens';\n`, `export { validateClaims } from './tokens';\n`, `export const CLOCK_SKEW = 30;\n`].join('\n'),
    },
  ];
  return raw.map((f) => ({ ...f, hunks: lineDiff(f.original, f.modified) }));
}

export function movesDemo(): Fixture {
  const files = demoFiles();
  const annotations: Annotation[] = [
    { path: 'src/auth/session.ts', side: 'new', line: 6, kind: 'finding', severity: 'medium', text: 'Bugs: isExpired ignores a negative skew.' },
    { path: 'src/auth/session.ts', side: 'new', line: 11, kind: 'comment', text: 'You: should signOut clear the refresh timer too?' },
    { path: 'src/auth/session.ts', side: 'old', line: 7, kind: 'finding', severity: 'low', text: 'Style: finding on a removed line (old side).' },
    { path: 'src/auth/tokens.ts', side: 'new', line: 9, kind: 'finding', severity: 'high', text: 'Bugs: atob throws on base64url input.' },
    { path: 'src/auth/tokens.ts', side: 'new', line: 15, kind: 'comment', text: 'You: nice, `!== undefined` fixes nbf = 0.' },
  ];
  return { name: 'Moves demo (3 files)', files, annotations, moves: detectMoves(files) };
}

// ---------------------------------------------------------------------------
// Large generated file. Hunks are emitted directly rather than diffed.

function rng(seed: number) {
  return () => {
    seed = (seed * 1664525 + 1013904223) >>> 0;
    return seed / 2 ** 32;
  };
}

const words = ['user', 'order', 'cache', 'token', 'item', 'queue', 'retry', 'limit', 'buffer', 'record', 'event', 'value'];

function fn(r: () => number, i: number): string[] {
  const a = words[Math.floor(r() * words.length)];
  const b = words[Math.floor(r() * words.length)];
  return [
    `export function ${a}${i}(${b}: ${cap(b)}, opts: Options = {}): Result<${cap(a)}> {`,
    `  const start = performance.now();`,
    `  if (!${b} || opts.skip) return { ok: false, reason: 'missing ${b}' };`,
    `  const ${a} = lookup${cap(a)}(${b}.id, opts.limit ?? ${Math.floor(r() * 100)});`,
    `  for (const entry of ${a}.entries) {`,
    `    if (entry.expired) continue; // stale ${b} entries are skipped`,
    `    metrics.observe('${a}.${b}', performance.now() - start);`,
    `  }`,
    `  return { ok: true, value: ${a} };`,
    `}`,
    ``,
  ];
}

const cap = (s: string) => s[0].toUpperCase() + s.slice(1);

export function largeFixture(lines: number): Fixture {
  const r = rng(lines);
  const old: string[] = [`// Generated ${lines}-line file for the viewer spike.`, `import { metrics } from './metrics';`, ``];
  let i = 0;
  while (old.length < lines) old.push(...fn(r, i++));
  old.length = lines;

  // Every ~120 lines: change one line, insert two, or delete one.
  const neu: string[] = [];
  const hunks: Hunk[] = [];
  const annotations: Annotation[] = [];
  const path = `src/generated/large-${lines}.ts`;
  for (let o = 0; o < old.length; o++) {
    const roll = o > 10 && o % 120 === 60 ? Math.floor(r() * 3) : -1;
    if (roll === 0) {
      hunks.push({ oldStart: o + 1, oldLines: 1, newStart: neu.length + 1, newLines: 1 });
      neu.push(old[o].replace('const', 'let') + ' // changed');
    } else if (roll === 1) {
      neu.push(old[o]);
      hunks.push({ oldStart: o + 1, oldLines: 0, newStart: neu.length + 1, newLines: 2 });
      neu.push(`  // inserted at old line ${o + 1}`, `  assertInvariant(${o});`);
    } else if (roll === 2) {
      hunks.push({ oldStart: o + 1, oldLines: 1, newStart: neu.length, newLines: 0 });
    } else {
      neu.push(old[o]);
    }
    if (o % 100 === 50) {
      annotations.push({ path, side: 'new', line: neu.length, kind: o % 300 === 50 ? 'comment' : 'finding', severity: 'medium', text: `Generated annotation near old line ${o + 1}` });
    }
  }
  const file: FileDiff = { path, original: old.join('\n') + '\n', modified: neu.join('\n') + '\n', hunks };
  return { name: `Large file (${lines.toLocaleString()} lines)`, files: [file], annotations, moves: [] };
}

export function fixtureFromGit(name: string, files: FileDiff[]): Fixture {
  return { name, files, annotations: [], moves: detectMoves(files) };
}
