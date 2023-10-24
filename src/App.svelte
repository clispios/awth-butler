<script lang="ts">
    import Profiles from "./lib/Profiles.svelte";
    import { DarkMode } from "flowbite-svelte";
    import { Navbar } from "flowbite-svelte";
</script>

<div>
    <Navbar href="/">
        <div class="flex left-align">
            <img src="/icon.png" class="mr-3 h-20 sm:h-15" alt="Svelte Logo" />
            <span
                class="self-center whitespace-nowrap text-3xl font-semibold dark:text-white"
                >AWth Butler</span
            >
        </div>
        <div class="right-align">
            <DarkMode />
        </div>
    </Navbar>
    <main class="main-body">
        <div class="row">
            <Profiles />
        </div>
    </main>
</div>
