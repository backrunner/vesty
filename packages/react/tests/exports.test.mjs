import assert from "node:assert/strict";
import {
  VestyBridgeProvider,
  useVestyBridge,
  useVestyParamEdit,
  useVestySnapshot,
  useVestySnapshotStore
} from "../dist/index.js";

assert.equal(typeof VestyBridgeProvider, "function");
assert.equal(typeof useVestyBridge, "function");
assert.equal(typeof useVestyParamEdit, "function");
assert.equal(typeof useVestySnapshot, "function");
assert.equal(typeof useVestySnapshotStore, "function");

console.log("@vesty/react exports ok");
