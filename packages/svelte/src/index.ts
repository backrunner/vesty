import { readable, type Readable } from "svelte/store";
import {
  createSnapshotStore,
  type PluginSnapshot,
  type SnapshotStoreOptions,
  type VestyBridge
} from "@vesty/plugin-ui";

export interface VestyParamEditActions {
  begin(gestureId?: string): Promise<void>;
  perform(normalized: number, gestureId?: string): Promise<void>;
  end(gestureId?: string): Promise<void>;
  set(normalized: number, gestureId?: string): Promise<void>;
  format(normalized: number): Promise<string>;
  parse(text: string): Promise<number>;
}

export function vestySnapshotStore<TSnapshot extends PluginSnapshot = PluginSnapshot>(
  bridge: VestyBridge,
  initialSnapshot?: TSnapshot,
  options: SnapshotStoreOptions = {}
): Readable<TSnapshot | undefined> {
  return readable<TSnapshot | undefined>(initialSnapshot, (set) => {
    const store = createSnapshotStore(bridge, initialSnapshot, options);
    const unsubscribe = store.subscribe(set);
    if (!store.getSnapshot()) void store.refresh().catch(() => undefined);

    return () => {
      unsubscribe();
      store.dispose();
    };
  });
}

export function vestyParamEdit(id: string, bridge: VestyBridge): VestyParamEditActions {
  return {
    begin: (gestureId) => bridge.beginParamEdit(id, gestureId),
    perform: (normalized, gestureId) => bridge.performParamEdit(id, normalized, gestureId),
    end: (gestureId) => bridge.endParamEdit(id, gestureId),
    set: (normalized, gestureId) => bridge.setParam(id, normalized, gestureId),
    format: (normalized) => bridge.formatParam(id, normalized),
    parse: (text) => bridge.parseParam(id, text)
  };
}
