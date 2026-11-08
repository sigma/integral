import { useCallback, useEffect, useState } from "react";
import {
  bindInstance,
  loadDefaultTone,
  subscribeInstance,
  subscribeRefusals,
  type InstanceInfo,
  type PluginBinding,
  type PluginBridge,
} from "./plugin";

/** The plugin instance a page is the editor of. */
export interface PluginInstance {
  /** What the plugin last said about it; `null` until it has. */
  info: InstanceInfo | null;
  /** Why the plugin last refused a bind or a gesture; cleared when it next reports the instance. */
  refusal: string | null;
  /** Bind the instance; the plugin answers with the new `info`, or an error. */
  bind: (binding: PluginBinding) => void;
  /** Load the plugin's default tone onto the bound Part, which holds another tone type. */
  loadDefaultTone: () => void;
}

/**
 * The instance the plugin behind `bridge` reports, or nothing for `null` (a
 * browser). Mount before the page says `ready` ({@link pluginPorts}), which
 * makes the plugin report.
 */
export function usePluginInstance(bridge: PluginBridge | null): PluginInstance {
  const [info, setInfo] = useState<InstanceInfo | null>(null);
  const [refusal, setRefusal] = useState<string | null>(null);
  useEffect(() => {
    if (!bridge) return;
    const stopInstance = subscribeInstance(bridge, (instance) => {
      setInfo(instance);
      setRefusal(null);
    });
    const stopRefusals = subscribeRefusals(bridge, setRefusal);
    return () => {
      stopInstance();
      stopRefusals();
    };
  }, [bridge]);
  const bind = useCallback(
    (binding: PluginBinding) => {
      if (bridge) bindInstance(bridge, binding);
    },
    [bridge],
  );
  const loadDefault = useCallback(() => {
    if (bridge) loadDefaultTone(bridge);
  }, [bridge]);
  return { info, refusal, bind, loadDefaultTone: loadDefault };
}
