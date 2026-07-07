<script lang="ts">
  let { name, onclose }: { name: string; onclose: () => void } = $props();
</script>

<p>{name}</p>
