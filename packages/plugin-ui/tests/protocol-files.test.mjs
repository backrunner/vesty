import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const index = await readFile(new URL("../dist/index.d.ts", import.meta.url), "utf8");
assert.match(index, /export type \{[\s\S]*BridgeReadyPayload[\s\S]*\} from "\.\/protocol"/);
assert.match(index, /export type \{[\s\S]*ParamMidiMapping[\s\S]*\} from "\.\/protocol"/);
assert.match(index, /export type \{ JsonValue \} from "\.\/serde_json\/JsonValue"/);

const protocolIndex = await readFile(new URL("../dist/protocol/index.d.ts", import.meta.url), "utf8");
assert.match(protocolIndex, /BridgePacket/);
assert.match(protocolIndex, /ParamMidiMapping/);
assert.match(protocolIndex, /ParamSpec/);

const ready = await readFile(new URL("../dist/protocol/BridgeReadyPayload.d.ts", import.meta.url), "utf8");
assert.match(ready, /params: Array<ParamSpec>/);
assert.match(ready, /snapshot: PluginSnapshot/);

const paramSpec = await readFile(new URL("../dist/protocol/ParamSpec.d.ts", import.meta.url), "utf8");
assert.match(paramSpec, /defaultNormalized: number/);
assert.match(paramSpec, /stepCount: number \| null/);
assert.match(paramSpec, /midiMappings: Array<ParamMidiMapping>/);
assert.doesNotMatch(paramSpec, /default_normalized/);
assert.doesNotMatch(paramSpec, /step_count/);

const paramMidiMapping = await readFile(
  new URL("../dist/protocol/ParamMidiMapping.d.ts", import.meta.url),
  "utf8"
);
assert.match(paramMidiMapping, /controller: number/);
assert.match(paramMidiMapping, /channel: number \| null/);

const paramKind = await readFile(new URL("../dist/protocol/ParamKind.d.ts", import.meta.url), "utf8");
assert.match(paramKind, /"float"/);
assert.match(paramKind, /"bool"/);
assert.match(paramKind, /"choice"/);

console.log("@vesty/plugin-ui protocol exports ok");
