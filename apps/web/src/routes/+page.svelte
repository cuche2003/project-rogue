<svelte:head>
  <title>Project Rogue</title>
  <meta
    name="description"
    content="A stylish roguelike adventure powered by a Rust engine."
  />
</svelte:head>

<main>
  <div class="eyebrow">PROJECT ROGUE</div>
  <h1>Enter the <span>unknown.</span></h1>
  <p class="intro">
    A handcrafted roguelike built around a deterministic Rust engine and a
    fast, expressive web front end.
  </p>
  <button type="button">Begin run <span aria-hidden="true">→</span></button>
</main>

<style>
  :global(*) {
    box-sizing: border-box;
  }

  :global(body) {
    margin: 0;
    background: #0b0b10;
    color: #f5f1e8;
    font-family: Inter, ui-sans-serif, system-ui, sans-serif;
  }

  main {
    display: grid;
    min-height: 100vh;
    place-content: center;
    padding: 2rem;
    background:
      radial-gradient(circle at 70% 35%, rgb(127 68 255 / 0.2), transparent 28rem),
      radial-gradient(circle at 20% 80%, rgb(255 105 64 / 0.12), transparent 24rem),
      #0b0b10;
  }

  .eyebrow {
    margin-bottom: 1.5rem;
    color: #a99cff;
    font-size: 0.75rem;
    font-weight: 700;
    letter-spacing: 0.28em;
  }

  h1 {
    max-width: 12ch;
    margin: 0;
    font-size: clamp(3.5rem, 10vw, 8rem);
    line-height: 0.9;
    letter-spacing: -0.07em;
  }

  h1 span {
    color: #ff8a68;
  }

  .intro {
    max-width: 34rem;
    margin: 2rem 0;
    color: #aaa8b3;
    font-size: 1.1rem;
    line-height: 1.6;
  }

  button {
    width: fit-content;
    border: 0;
    border-radius: 999px;
    padding: 0.9rem 1.3rem;
    background: #f5f1e8;
    color: #17151d;
    cursor: pointer;
    font: inherit;
    font-weight: 700;
    transition: transform 160ms ease, background 160ms ease;
  }

  button:hover {
    background: #ff8a68;
    transform: translateY(-2px);
  }

  button span {
    margin-left: 0.5rem;
  }
</style>
