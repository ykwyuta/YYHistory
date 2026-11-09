import test from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';

const buildUrl = new URL('./build.mjs', import.meta.url).href;

// Run the real entry point with a simulated host and child process result.
// No compiler installation or network access is needed for these regressions.
function simulate(platform, result) {
  return spawnSync(process.execPath, ['--input-type=module', '-e', `
    import childProcess from 'node:child_process';
    import { syncBuiltinESMExports } from 'node:module';
    Object.defineProperty(process, 'platform', { value: ${JSON.stringify(platform)} });
    childProcess.spawnSync = (command, args) => {
      console.log(JSON.stringify({ command, args }));
      return ${result};
    };
    syncBuiltinESMExports();
    await import(${JSON.stringify(buildUrl)});
  `], { encoding: 'utf8' });
}

test('Linux builds use the shell entry point that provisions Rust', () => {
  const result = simulate('linux', '{ status: 0 }');
  assert.equal(result.status, 0, result.stderr);
  assert.deepEqual(JSON.parse(result.stdout), { command: 'bash', args: ['build.sh'] });
});

test('shell build failures reach the build service', () => {
  assert.equal(simulate('linux', '{ status: 23 }').status, 23);
});

test('missing build tools produce a visible diagnostic', () => {
  for (const [platform, command] of [['linux', 'bash build.sh'], ['win32', 'cargo']]) {
    const result = simulate(platform, '{ status: null, error: new Error("ENOENT") }');
    assert.notEqual(result.status, 0);
    assert.ok(result.stderr.includes(`Could not start ${command}: ENOENT`), result.stderr);
  }
});
