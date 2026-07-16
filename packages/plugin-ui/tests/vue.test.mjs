import assert from "node:assert/strict";
import { useVestyParamEdit, useVestySnapshot } from "../dist/vue.js";

assert.equal(typeof useVestyParamEdit, "function");
assert.equal(typeof useVestySnapshot, "function");

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

  const actions = useVestyParamEdit("gain", bridge);
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

console.log("vesty-plugin-ui/vue exports ok");
