<script lang="ts">
  import { onMount } from 'svelte';
  import type { SvedocsPage } from 'svedocs/core';
  import type { SvedocsThemeContext } from 'svedocs/theme/types';
  import { resolveLocalizedHref } from 'svedocs/theme/headless';

  export let page: SvedocsPage;
  export let context: SvedocsThemeContext;

  let canvas: HTMLCanvasElement;

  const stages = [
    ['landing.stageHost', 'landing.stageHostDescription'],
    ['landing.stageAdapter', 'landing.stageAdapterDescription'],
    ['landing.stageKernel', 'landing.stageKernelDescription'],
    ['landing.stageEditor', 'landing.stageEditorDescription']
  ];

  const guides = [
    ['01', 'landing.cardStart', 'landing.cardStartDescription', '/docs'],
    ['02', 'landing.cardRealtime', 'landing.cardRealtimeDescription', '/docs/concepts/realtime-safety'],
    ['03', 'landing.cardWeb', 'landing.cardWebDescription', '/docs/guides/web-ui'],
    ['04', 'landing.cardShip', 'landing.cardShipDescription', '/docs/tooling/packaging']
  ];

  onMount(() => {
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    let frame = 0;
    let animation = 0;
    let width = 0;
    let height = 0;
    const motion = window.matchMedia('(prefers-reduced-motion: reduce)');
    let reduceMotion = motion.matches;

    const resize = () => {
      const rect = canvas.getBoundingClientRect();
      const scale = Math.min(window.devicePixelRatio || 1, 2);
      width = Math.max(1, rect.width);
      height = Math.max(1, rect.height);
      canvas.width = Math.round(width * scale);
      canvas.height = Math.round(height * scale);
      ctx.setTransform(scale, 0, 0, scale, 0, 0);
    };

    const drawWave = (y: number, amplitude: number, frequency: number, color: string, offset: number) => {
      ctx.beginPath();
      for (let x = 0; x <= width; x += 3) {
        const envelope = Math.sin(Math.PI * (x / width));
        const signal = Math.sin(x * frequency + frame * 0.018 + offset)
          + Math.sin(x * frequency * 0.31 - frame * 0.011) * 0.34;
        const point = y + signal * amplitude * envelope;
        if (x === 0) ctx.moveTo(x, point);
        else ctx.lineTo(x, point);
      }
      ctx.strokeStyle = color;
      ctx.lineWidth = 1.25;
      ctx.stroke();
    };

    const render = () => {
      ctx.clearRect(0, 0, width, height);
      ctx.globalAlpha = 0.22;
      ctx.strokeStyle = '#53615b';
      ctx.lineWidth = 1;
      for (let y = 32; y < height; y += 48) {
        ctx.beginPath();
        ctx.moveTo(0, y + 0.5);
        ctx.lineTo(width, y + 0.5);
        ctx.stroke();
      }
      ctx.globalAlpha = 0.8;
      drawWave(height * 0.43, 36, 0.026, '#62e6a7', 0);
      ctx.globalAlpha = 0.58;
      drawWave(height * 0.61, 24, 0.041, '#ff8a5b', 1.7);
      ctx.globalAlpha = 1;
      frame += 1;
      if (!reduceMotion) animation = requestAnimationFrame(render);
    };

    const handleResize = () => {
      resize();
      if (reduceMotion) render();
    };

    const handleMotion = (event: MediaQueryListEvent) => {
      reduceMotion = event.matches;
      cancelAnimationFrame(animation);
      frame = 0;
      render();
    };

    resize();
    window.addEventListener('resize', handleResize);
    motion.addEventListener('change', handleMotion);
    render();

    return () => {
      cancelAnimationFrame(animation);
      window.removeEventListener('resize', handleResize);
      motion.removeEventListener('change', handleMotion);
    };
  });
</script>

<div class="landing-shell">
  <section class="landing-hero" aria-labelledby="vesty-title">
    <canvas bind:this={canvas} class="signal-canvas" aria-hidden="true"></canvas>
    <div class="hero-status">
      <span>{context.t('landing.status')}</span>
      <span>44.1—192 kHz</span>
      <span>f32 / f64</span>
    </div>
    <div class="hero-copy">
      <p class="eyebrow"><span aria-hidden="true"></span>{context.t('landing.eyebrow')}</p>
      <h1 id="vesty-title">{page.title}</h1>
      <p class="hero-description">{context.t('landing.description')}</p>
      <div class="hero-actions">
        <a class="action-primary" href={resolveLocalizedHref('/docs', context)}>{context.t('landing.docs')} <span aria-hidden="true">→</span></a>
        <a class="action-secondary" href="https://github.com/orchiliao/vesty">{context.t('landing.github')} <span aria-hidden="true">↗</span></a>
      </div>
    </div>
    <dl class="hero-metrics">
      <div><dt>{context.t('landing.scope')}</dt><dd>{context.t('landing.scopeValue')}</dd></div>
      <div><dt>{context.t('landing.bridge')}</dt><dd>{context.t('landing.bridgeValue')}</dd></div>
      <div><dt>{context.t('landing.editor')}</dt><dd>{context.t('landing.editorValue')}</dd></div>
    </dl>
    <div class="hero-command" aria-label="Install command"><span>$</span><code>cargo add vesty</code></div>
  </section>

  <section class="signal-section">
    <div class="section-heading">
      <p>HOST → KERNEL → UI</p>
      <h2>{context.t('landing.signalTitle')}</h2>
      <span>{context.t('landing.signalDescription')}</span>
    </div>
    <ol class="signal-stages">
      {#each stages as stage, index}
        <li>
          <span class="stage-index">0{index + 1}</span>
          <div><strong>{context.t(stage[0])}</strong><small>{context.t(stage[1])}</small></div>
          {#if index < stages.length - 1}<i aria-hidden="true"></i>{/if}
        </li>
      {/each}
    </ol>
  </section>

  <section class="explore-section">
    <div class="section-heading compact">
      <p>DOCUMENTATION MAP</p>
      <h2>{context.t('landing.explore')}</h2>
      <span>{context.t('landing.exploreDescription')}</span>
    </div>
    <div class="guide-grid">
      {#each guides as guide}
        <a href={resolveLocalizedHref(guide[3], context)}>
          <span>{guide[0]}</span>
          <strong>{context.t(guide[1])}</strong>
          <small>{context.t(guide[2])}</small>
          <i aria-hidden="true">→</i>
        </a>
      {/each}
    </div>
  </section>

  <section class="contract-section">
    <p>{context.t('landing.contract')}</p>
    <h2>{context.t('landing.contractTitle')}</h2>
    <span>{context.t('landing.contractDescription')}</span>
    <a href={resolveLocalizedHref('/docs/concepts/realtime-safety', context)}>{context.t('landing.contractAction')} <i aria-hidden="true">→</i></a>
  </section>
</div>
