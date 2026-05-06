<script lang="ts">
    import { onMount } from "svelte";

    export let to;

    onMount(() => {
        window.location.replace(to);
    });
</script>
