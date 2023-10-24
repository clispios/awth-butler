<script lang="ts">
    import { invoke } from "@tauri-apps/api/tauri";
    import { listen, type UnlistenFn } from "@tauri-apps/api/event";
 import { open as openbrow } from '@tauri-apps/api/shell';
 import { slide } from "svelte/transition";
 import {
        Spinner,
        Table,
        TableBody,
        TableBodyCell,
        TableBodyRow,
        TableHead,
        TableHeadCell,
        Toast,
        Radio,
        GradientButton,
    } from "flowbite-svelte";
    import { FireOutline } from "flowbite-svelte-icons";
 import toastSoundImport from './../assets/toastSound.wav';
 import toastCanceledSoundImport from './../assets/toastCanceled.wav';
 import toastSuccessSoundImport from './../assets/toastSuccess.mp3';


 let open: boolean;
 let counter: number;
    let counter_color: string;
 let spinner = false;
    let running_login_status = false;
    let runningLoginPid: number;
    let unlisten: UnlistenFn;
    let finishedSuccessfully = false;
    let processTimedOut = false;

    function timeout() {
        if (counter < 21) {
            counter_color = "text-red-400";
        }
        if (--counter > 0) return setTimeout(timeout, 1000);
    }

    let selectedProf: string;
    let toast_auth_code: string;
    let toast_auth_url: string;
    interface auth_payload {
        event_type: string;
        auth_code: string;
        auth_url: string;
        process_pid: number;
    }
    let profiles: string[];

    function set_prof(prof: string) {
        selectedProf = prof;
    }

    function formatDate(dateString: string) {
        if (dateString === "never authenticated") {
            return dateString;
        } else if (dateString !== null) {
            let d = new Date(Date.parse(dateString));
            if (d.getTime() <= new Date().getTime()) {
                return d.toLocaleDateString("en-us", {
                    year: "numeric",
                    month: "short",
                    day: "numeric",
                    hour: "numeric",
                    minute: "numeric",
                    second: "numeric",
                    timeZoneName: "short",
                });
            }
            return "Expired";
        } else {
            return "";
        }
    }

    let toastStartSound = () => {
        new Audio(toastSoundImport).play();
    }
    let toastCanceledSound = () => {
        new Audio(toastCanceledSoundImport).play();
    }

    let toastSuccessSound = () => {
        new Audio(toastSuccessSoundImport).play();
    }

    async function get_profile_exp(prof: string): Promise<string> {
        return await invoke("get_profile_exp", { prof });
    }
    async function get_profiles() {
        selectedProf = undefined;
        const full_profiles: string[] = await invoke("get_profiles");
        profiles = full_profiles.map((v): string => {
            return v.split("profile ").at(-1) ?? "";
        });
    }

    function start_event(event: auth_payload) {
        toast_auth_code = event.auth_code;
        toast_auth_url = event.auth_url;
        openbrow(toast_auth_url);
        counter = 60;
        open = true;
        counter_color = "text-indigo-400";
        toastStartSound();
        timeout();
    }
    function timeout_event() {
        finishedSuccessfully = false;
        processTimedOut = true;
        open = false;
        counter = 0;
        running_login_status = false;
        runningLoginPid = 0;
    }
    function success_event() {
        finishedSuccessfully = true;
        processTimedOut = false;
        open = false;
        counter = 0;
        running_login_status = false;
        runningLoginPid = 0;
    }
    function wrap_up(wrap_type: string) {
        finishedSuccessfully = false;
        processTimedOut = false;
        unlisten();
        if(wrap_type === "timeout"){
            toastCanceledSound();
        } else {
            toastSuccessSound();
        }
    }

    async function start_sso_login(prof: string) {
        spinner = true;
        unlisten = await listen<auth_payload>("sso-login", (event) => {
            if (event.payload.event_type === "start") {
                spinner = false;
                start_event(event.payload);
            } else if (event.payload.event_type === "success") {
                spinner = false;
                success_event();
            } else {
                spinner = false;
                timeout_event();
            }
        });
        runningLoginPid = await invoke("do_sso_login", { prof });
        running_login_status = true;
    }

    async function sso_cancel() {
        if (running_login_status) {
            await invoke("do_sso_cancel", { runningLoginPid });
        }
        open = false;
        counter = 0;
        running_login_status = false;
        runningLoginPid = 0;

        unlisten();
        toastCanceledSound();
    }
    $: () => {
        if (unlisten) {
            unlisten();
        }
    };
    $: get_profiles();
</script>

<div>
    {#if open}
        <Toast
            color="indigo"
            class="shadow-lg shadow-indigo-500/50"
            position="top-right"
            transition={slide}
            dismissable={false}
            bind:open
        >
            <FireOutline slot="icon" class="w-5 h-5" />
            <p>Auth Code: <span class="font-bold">{toast_auth_code}</span></p>
            <p>
                <GradientButton
                    size="xs"
                    color="teal"
                    on:click={() => {openbrow(toast_auth_url)}}>Manually open browser</GradientButton
                                                >
            </p>
            <p>
                <span class="{counter_color} font-bold">{counter}</span>s until
                canceled.
            </p>
            <br />
            <GradientButton
                size="xs"
                class="right-align"
                color="red"
                on:click={sso_cancel}>Cancel</GradientButton
            >
        </Toast>
        <br />
    {:else if finishedSuccessfully}
        {wrap_up("success")}
    {:else if processTimedOut}
        {wrap_up("timeout")}
    {/if}
    <div class="flex gap-4">
        <GradientButton color="cyanToBlue" on:click={get_profiles}
            >Reload Profile List</GradientButton
        >
        {#if selectedProf}
            <GradientButton
                class="right-align"
                color="purpleToBlue"
                disabled={running_login_status}
                on:click={() => {
                         start_sso_login(selectedProf);
                         }}>
                {#if spinner}
                    <Spinner class="mr-3" size="4" color="white" />
                {/if}
                Authenticate {selectedProf}</GradientButton>
        {:else}
            <GradientButton
                class="right-align"
                color="purpleToBlue"
                disabled="true">Select a profile to authenticate</GradientButton
            >
        {/if}
    </div>
    <br /><br />
    <Table hoverable={true}>
        <TableHead>
            <TableHeadCell>Selection</TableHeadCell>
            <TableHeadCell>Profile name</TableHeadCell>
            <TableHeadCell>Auth Status</TableHeadCell>
            <TableHeadCell>Expires In</TableHeadCell>
        </TableHead>
        <TableBody>
            {#if profiles === undefined || !profiles.length}
                <p>Your AWS config is empty or doesn't exist!</p>
            {:else}
                {#each profiles as prof}
                    {#await get_profile_exp(prof)}
                        <br />
                    {:then exp}
                        <TableBodyRow
                            on:click={() => {
                                set_prof(prof);
                            }}
                        >
                            <TableBodyCell class="!p-4">
                                <Radio
                                    bind:group={selectedProf}
                                    color="cyan"
                                    value={prof}
                                />
                            </TableBodyCell>
                            <TableBodyCell>{prof}</TableBodyCell>
                            <TableBodyCell>n/a</TableBodyCell>
                            <TableBodyCell>{formatDate(exp)}</TableBodyCell>
                        </TableBodyRow>
                    {/await}
                {/each}
            {/if}
        </TableBody>
    </Table>
</div>
