<script lang="ts">
  let {
    on,
    disabled = false,
    label,
    onchange,
  }: {
    on: boolean;
    disabled?: boolean;
    label: string;
    onchange: () => void;
  } = $props();
</script>

<button
  type="button"
  class="switch"
  class:on
  role="switch"
  aria-checked={on}
  aria-label={label}
  {disabled}
  onclick={onchange}
></button>

<style>
  /* M3 switch */
  .switch {
    width: 52px;
    height: 32px;
    border-radius: 99px;
    background: var(--c-highest);
    box-shadow: inset 0 0 0 2px var(--outline);
    position: relative;
    flex: none;
    border: 0;
    padding: 0;
  }
  .switch::after {
    content: "";
    position: absolute;
    left: 8px;
    top: 8px;
    width: 16px;
    height: 16px;
    border-radius: 99px;
    background: var(--outline);
    transition: all 0.2s;
  }
  .switch.on {
    background: var(--primary);
    box-shadow: none;
  }
  .switch.on::after {
    left: 24px;
    top: 4px;
    width: 24px;
    height: 24px;
    background: var(--on-primary);
  }
  .switch:disabled {
    opacity: 0.38;
  }
</style>
