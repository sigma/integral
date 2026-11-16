/** An Init flow around a fake service, for the views that offer "Init…" items. */

import { act, type ReactNode } from "react";
import type { InitScope } from "../init";
import { InitProvider } from "../InitFlow";
import type { IntegraService } from "../integra";
import { button, click } from "./dom";

/**
 * Makes `service`'s Inits record their scope and wait for `finish`, and
 * returns a wrapper that puts a view under an `InitProvider` on it.
 */
export function fakeInit(service: IntegraService) {
  const asked: InitScope[] = [];
  let answer: (() => void) | undefined;
  service.initialize = (scope) => {
    asked.push(scope);
    return new Promise((resolve) => {
      answer = () => resolve(undefined);
    });
  };
  const mixer = { reload: () => {} };
  return {
    /** The scope of every Init the device was asked for. */
    asked,
    /** `node` under the flow. */
    wrap: (node: ReactNode) => (
      <InitProvider service={service} mixer={mixer}>
        {node}
      </InitProvider>
    ),
    /** The device answers "finished" to the Init running. */
    finish: () =>
      act(async () => {
        answer?.();
      }),
  };
}

/** Clicks the "Init…" item named `name` in `scope`, then its confirmation's Init. */
export async function confirmInit(scope: ParentNode, name: string): Promise<void> {
  click(button(scope, name));
  await act(async () => click(button(document.body, "Init")));
}
