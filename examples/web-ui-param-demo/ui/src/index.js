const bridge = window.__VESTY__;
const mix = document.querySelector("#mix");
const value = document.querySelector("#value");
const bridgeStatus = document.querySelector("#bridge-status");
const peak = document.querySelector("#peak");
const meterValue = document.querySelector("#meter-value");
let editing = false;
let unsubscribeParamChanged;

function setNormalized(normalized) {
  const next = Math.min(1, Math.max(0, Number(normalized) || 0));
  if (mix) mix.value = String(next);
  if (value) value.value = next.toFixed(3);
}

bridge?.ready()
  .then((payload) => {
    if (bridgeStatus) {
      bridgeStatus.value = `ready:${payload.pluginName ?? "unknown"}`;
    }
    const current = payload.paramValues?.find((param) => param.id === "mix")?.normalized;
    setNormalized(current ?? 0.5);
    unsubscribeParamChanged = bridge.subscribe("param.changed", (event) => {
      if (event?.id === "mix") setNormalized(event.normalized);
    });
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
  setNormalized(normalized);
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

window.addEventListener("pagehide", () => unsubscribeParamChanged?.(), { once: true });
