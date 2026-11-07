/** A bridge standing in for the plugin, for page tests. */

import type { PageMessage, PluginBridge, PluginMessage } from "../plugin";

/** A bridge standing in for the plugin: records what the page posts and
 *  delivers what the test sends. */
export function fakeBridge() {
  const posted: PageMessage[] = [];
  const listeners = new Set<(message: string) => void>();
  let answer: ((message: PageMessage) => PluginMessage[]) | null = null;
  const deliver = (message: PluginMessage) => {
    for (const l of [...listeners]) l(JSON.stringify(message));
  };
  const bridge: PluginBridge = {
    post(text) {
      const message = JSON.parse(text) as PageMessage;
      posted.push(message);
      for (const reply of answer?.(message) ?? []) queueMicrotask(() => deliver(reply));
    },
    subscribe(listener) {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
  };
  return {
    bridge,
    posted,
    deliver,
    /** Answer each posted message with what `reply` returns. */
    answerWith(reply: (message: PageMessage) => PluginMessage[]) {
      answer = reply;
    },
    rawDeliver(text: string) {
      for (const l of [...listeners]) l(text);
    },
  };
}
