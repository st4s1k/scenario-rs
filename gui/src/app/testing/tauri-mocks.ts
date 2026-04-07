/**
 * Shared Tauri API mocking utilities for Angular tests.
 *
 * All Tauri API calls (@tauri-apps/api/core invoke, @tauri-apps/api/event listen,
 * @tauri-apps/plugin-dialog open, etc.) go through window.__TAURI_INTERNALS__.invoke.
 * This module spies on that single entry-point and provides helpers for
 * configuration and assertions.
 *
 * Usage in spec files:
 *   import { setupTauriMock, TauriTestHarness } from '../testing/tauri-mocks';
 *
 *   let tauri: TauriTestHarness;
 *   beforeEach(() => { tauri = setupTauriMock(); });
 */

export interface TauriTestHarness {
  /** The Jasmine spy on __TAURI_INTERNALS__.invoke */
  invokeSpy: jasmine.Spy;

  /** Set (or override) the mock response for a given invoke command.
   *  `response` can be a plain value or a function `(args?) => value`. */
  setResponse(cmd: string, response: any): void;

  /** Emit a Tauri event to a listener registered via listen(). */
  emitEvent(event: string, payload: any): void;

  /** Assert that a specific invoke command was called (optionally with matching args). */
  expectInvoked(cmd: string, args?: Record<string, any>): void;

  /** Assert that a specific invoke command was NOT called. */
  expectNotInvoked(cmd: string): void;
}

/**
 * Set up a spy on window.__TAURI_INTERNALS__.invoke.
 *
 * @param defaults Optional map of command → response. Values can be plain values
 *   or functions `(args?) => value` for dynamic responses.
 */
export function setupTauriMock(defaults?: Record<string, any>): TauriTestHarness {
  const responses = new Map<string, any>(Object.entries(defaults || {}));
  const eventHandlers = new Map<string, number>();

  const invokeSpy = spyOn(
    (window as any).__TAURI_INTERNALS__,
    'invoke',
  ).and.callFake((cmd: string, args?: any) => {
    if (cmd === 'plugin:event|listen') {
      eventHandlers.set(args.event, args.handler);
      return Promise.resolve(1);
    }
    if (cmd === 'plugin:event|unlisten') {
      return Promise.resolve();
    }
    const response = responses.get(cmd);
    if (typeof response === 'function') {
      return Promise.resolve(response(args));
    }
    return Promise.resolve(response);
  });

  return {
    invokeSpy,
    setResponse(cmd: string, response: any) {
      responses.set(cmd, response);
    },
    emitEvent(event: string, payload: any) {
      const id = eventHandlers.get(event);
      if (id !== undefined && (window as any)[`_${id}`]) {
        (window as any)[`_${id}`]({ payload });
      }
    },
    expectInvoked(cmd: string, args?: Record<string, any>) {
      const calls = invokeSpy.calls.allArgs();
      const match = calls.find((a: any[]) => a[0] === cmd);
      expect(match)
        .withContext(`Expected invoke('${cmd}') to have been called`)
        .toBeTruthy();
      if (args) {
        expect(match![1]).toEqual(jasmine.objectContaining(args));
      }
    },
    expectNotInvoked(cmd: string) {
      const calls = invokeSpy.calls.allArgs();
      const match = calls.find((a: any[]) => a[0] === cmd);
      expect(match)
        .withContext(`Expected invoke('${cmd}') not to have been called`)
        .toBeFalsy();
    },
  };
}
