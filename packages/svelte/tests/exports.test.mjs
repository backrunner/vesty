import assert from "node:assert/strict";
import { vestyParamEdit, vestySnapshotStore } from "../dist/index.js";

assert.equal(typeof vestyParamEdit, "function");
assert.equal(typeof vestySnapshotStore, "function");

{
  const calls = [];
  const bridge = {
    beginParamEdit: async (id, gestureId) => calls.push(["begin", id, gestureId]),
    performParamEdit: async (id, normalized, gestureId) =>
      calls.push(["perform", id, normalized, gestureId]),
    endParamEdit: async (id, gestureId) => calls.push(["end", id, gestureId]),
    setParam: async (id, normalized, gestureId) => calls.push(["set", id, normalized, gestureId]),
    formatParam: async (id, normalized) => {
      calls.push(["format", id, normalized]);
      return `${normalized}`;
    },
    parseParam: async (id, text) => {
      calls.push(["parse", id, text]);
      return 0.5;
    }
  };

  const actions = vestyParamEdit("gain", bridge);
  await actions.begin("drag-1");
  await actions.perform(0.75, "drag-1");
  await actions.set(0.25, "drag-1");
  await actions.end("drag-1");

  assert.deepEqual(calls, [
    ["begin", "gain", "drag-1"],
    ["perform", "gain", 0.75, "drag-1"],
    ["set", "gain", 0.25, "drag-1"],
    ["end", "gain", "drag-1"]
  ]);
}

console.log("@vesty/svelte exports ok");
