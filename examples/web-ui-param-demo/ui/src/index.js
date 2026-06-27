const bridge = window.__VESTY__;
const mix = document.querySelector("#mix");
const value = document.querySelector("#value");
const bridgeStatus = document.querySelector("#bridge-status");
const peak = document.querySelector("#peak");
const meterValue = document.querySelector("#meter-value");
let editing = false;

bridge?.ready()
  .then((payload) => {
    if (bridgeStatus) {
      bridgeStatus.value = `ready:${payload.pluginName ?? "unknown"}`;
    }
  })
  .catch((error) => {
    if (bridgeStatus) {
      bridgeStatus.value = `error:${error?.code ?? "bridge"}`;
    }
  });

function begin(event) {
  if (!mix || editing) return;
  editing = true;
  mix.setPointerCapture(event.pointerId);
  bridge?.beginParamEdit("mix");
}

function perform() {
  if (!mix) return;
  const normalized = Number(mix.value);
  if (value) value.value = normalized.toFixed(3);
  bridge?.performParamEdit("mix", normalized);
}

function end(event) {
  if (!mix || !editing) return;
  editing = false;
  if (event && mix.hasPointerCapture(event.pointerId)) {
    mix.releasePointerCapture(event.pointerId);
  }
  bridge?.endParamEdit("mix");
}

mix?.addEventListener("pointerdown", begin);
mix?.addEventListener("input", perform);
mix?.addEventListener("pointerup", end);
mix?.addEventListener("pointercancel", end);
mix?.addEventListener("lostpointercapture", end);

bridge?.subscribe("meter.main", (frame) => {
  const peaks = Array.isArray(frame?.peaks) ? frame.peaks : [];
  const nextPeak = peaks.reduce((max, sample) => Math.max(max, Number(sample) || 0), 0);
  if (peak) peak.value = String(nextPeak);
  if (meterValue) meterValue.value = nextPeak.toFixed(3);
});
