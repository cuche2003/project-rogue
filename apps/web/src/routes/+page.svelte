<script lang="ts">
  import { onMount } from 'svelte';

  type Snapshot = {
    phase: string;
    turn: number;
    floor: number;
    floor_count: number;
    map: {
      width: number;
      height: number;
      tiles: (string | null)[];
      visible: boolean[];
    };
    entities: Entity[];
    items: Item[];
    player: {
      position: Position;
      health: { current: number; maximum: number };
    };
    inventory: Item[];
    messages: string[];
  };

  type Position = { x: number; y: number };
  type Entity = { position: Position; glyph: string; name: string; health?: { current: number; maximum: number } };
  type Item = { name: string; glyph: string; quantity: number; position: Position | null };
  type EngineModule = { default: () => Promise<void>; Engine: new () => { snapshot(): string; command(action: string): string } };

  let engine: InstanceType<EngineModule['Engine']> | undefined;
  let snapshot: Snapshot | undefined;
  let error = '';
  let loading = true;

  onMount(async () => {
    try {
      const module = (await import('../lib/engine/project_rogue_engine_wasm.js')) as unknown as EngineModule;
      await module.default();
      engine = new module.Engine();
      snapshot = JSON.parse(engine.snapshot()) as Snapshot;
    } catch (reason) {
      error = reason instanceof Error ? reason.message : String(reason);
    } finally {
      loading = false;
    }
  });

  function send(action: string) {
    if (!engine) return;
    try {
      const response = JSON.parse(engine.command(action)) as { snapshot: Snapshot };
      snapshot = response.snapshot;
    } catch (reason) {
      error = reason instanceof Error ? reason.message : String(reason);
    }
  }

  function tileAt(x: number, y: number) {
    if (!snapshot) return ' ';
    const index = y * snapshot.map.width + x;
    if (!snapshot.map.visible[index] && snapshot.map.tiles[index] === null) return ' ';
    const entity = snapshot.entities.find((item) => item.position.x === x && item.position.y === y);
    if (entity) return entity.glyph;
    const item = snapshot.items.find((item) => item.position?.x === x && item.position?.y === y);
    if (item) return item.glyph;
    return snapshot.map.tiles[index] === 'Wall' ? '█' : snapshot.map.tiles[index] === 'StairsDown' ? '>' : '·';
  }

  function mapRows() {
    const current = snapshot;
    if (!current) return [];
    return Array.from({ length: current.map.height }, (_, y) =>
      Array.from({ length: current.map.width }, (_, x) => tileAt(x, y)).join('')
    );
  }

  function keydown(event: KeyboardEvent) {
    const actions: Record<string, string> = {
      ArrowUp: 'move up',
      ArrowDown: 'move down',
      ArrowLeft: 'move left',
      ArrowRight: 'move right',
      w: 'move up',
      s: 'move down',
      a: 'move left',
      d: 'move right',
      '.': 'wait',
      g: 'pickup',
      '>': 'descend'
    };
    const action = actions[event.key];
    if (action) {
      event.preventDefault();
      send(action);
    }
  }
</script>

<svelte:head>
  <title>Project Rogue</title>
  <meta name="description" content="A deterministic roguelike powered by a Rust engine." />
</svelte:head>

<svelte:window onkeydown={keydown} />

<main>
  <header>
    <div>
      <div class="eyebrow">PROJECT ROGUE</div>
      <h1>RUN <span>01</span></h1>
    </div>
    {#if snapshot}
      <div class="stats">
        <span>FLOOR <b>{snapshot.floor}/{snapshot.floor_count}</b></span>
        <span>TURN <b>{snapshot.turn}</b></span>
        <span>HP <b>{snapshot.player.health.current}/{snapshot.player.health.maximum}</b></span>
      </div>
    {/if}
  </header>

  {#if loading}
    <section class="panel message">Loading engine…</section>
  {:else if error}
    <section class="panel message error">Engine failed: {error}</section>
  {:else if snapshot}
    <section class="game-layout">
      <div class="panel map-panel">
        <div class="panel-title"><span>DEPTH {snapshot.floor}</span><span class="phase">{snapshot.phase}</span></div>
        <pre class="map" aria-label="Game map">{mapRows().join('\n')}</pre>
        <div class="controls">ARROWS / WASD MOVE <span>·</span> . WAIT <span>·</span> G PICK UP <span>·</span> &gt; DESCEND</div>
      </div>

      <aside>
        <div class="panel log-panel">
          <div class="panel-title">COMBAT LOG</div>
          <div class="log">
            {#each snapshot.messages.slice(-8).reverse() as message}
              <p>{message}</p>
            {:else}
              <p class="muted">No events yet.</p>
            {/each}
          </div>
        </div>
        <div class="panel inventory-panel">
          <div class="panel-title">INVENTORY</div>
          {#if snapshot.inventory.length}
            {#each snapshot.inventory as item, index}
              <button class="item" type="button" onclick={() => send(`use ${index}`)}>
                <span>[{index + 1}] {item.glyph} {item.name}</span><b>×{item.quantity}</b>
              </button>
            {/each}
          {:else}
            <p class="muted empty">Empty</p>
          {/if}
        </div>
      </aside>
    </section>
  {/if}
</main>

<style>
  :global(*) { box-sizing: border-box; }
  :global(body) { margin: 0; background: #090a0e; color: #e6e8e6; font-family: 'Courier New', monospace; }
  main { min-height: 100vh; max-width: 1180px; margin: 0 auto; padding: 3rem 2rem; background: radial-gradient(circle at 15% 10%, #17232a 0, transparent 32rem); }
  header { display: flex; align-items: end; justify-content: space-between; gap: 2rem; margin-bottom: 2rem; border-bottom: 1px solid #273037; padding-bottom: 1.5rem; }
  .eyebrow, .panel-title, .controls, .stats { color: #83a8a1; font-size: .72rem; letter-spacing: .16em; }
  h1 { margin: .35rem 0 0; font-size: clamp(2rem, 7vw, 4.5rem); letter-spacing: .08em; line-height: .9; }
  h1 span, .phase { color: #d18b5c; }
  .stats { display: flex; gap: 1.2rem; flex-wrap: wrap; justify-content: end; }
  .stats b { color: #e6e8e6; font-weight: normal; }
  .game-layout { display: grid; grid-template-columns: minmax(0, 1fr) 280px; gap: 1rem; }
  .panel { border: 1px solid #273037; background: #0e1216; box-shadow: 0 1rem 4rem #0006; }
  .map-panel { min-width: 0; }
  .panel-title { display: flex; justify-content: space-between; border-bottom: 1px solid #273037; padding: .8rem 1rem; }
  .map { overflow: auto; margin: 0; padding: 1.5rem; color: #b7d0c7; font-size: clamp(.48rem, 1.25vw, .9rem); line-height: 1.25; letter-spacing: .08em; }
  .controls { border-top: 1px solid #273037; padding: .8rem 1rem; line-height: 1.8; }
  .controls span { color: #4c6260; margin: 0 .4rem; }
  aside { display: grid; align-content: start; gap: 1rem; }
  .log { min-height: 12rem; padding: .8rem 1rem; }
  .log p { margin: .55rem 0; color: #c2c8c3; font-size: .78rem; line-height: 1.45; }
  .muted { color: #5d6a68 !important; }
  .inventory-panel { padding-bottom: .5rem; }
  .item { display: flex; justify-content: space-between; width: 100%; border: 0; border-bottom: 1px solid #1b2327; background: transparent; color: #c2c8c3; padding: .8rem 1rem; font: inherit; font-size: .78rem; text-align: left; cursor: pointer; }
  .item:hover, .item:focus-visible { background: #17221f; color: #e6e8e6; outline: none; }
  .item b { color: #d18b5c; font-weight: normal; }
  .empty { padding: .5rem 1rem; font-size: .78rem; }
  .message { padding: 2rem; }
  .error { color: #d77969; }
  @media (max-width: 760px) {
    main { padding: 1.5rem 1rem; }
    header { align-items: start; flex-direction: column; gap: 1.2rem; }
    .stats { justify-content: start; }
    .game-layout { grid-template-columns: 1fr; }
    .map { font-size: .55rem; padding: 1rem; }
  }
</style>
