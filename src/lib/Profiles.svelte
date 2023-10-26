<script lang="ts">
    import { invoke } from "@tauri-apps/api/tauri";
    import { listen, type UnlistenFn } from "@tauri-apps/api/event";
    import { open as openbrow } from "@tauri-apps/api/shell";
    import { slide } from "svelte/transition";
    import {
        Badge,
        Spinner,
        Table,
        TableBody,
        TableBodyCell,
        TableBodyRow,
        TableHead,
        TableHeadCell,
        Toast,
        GradientButton,
    } from "flowbite-svelte";
    import {
        FireOutline,
        CheckCircleSolid,
        CloseCircleSolid,
    } from "flowbite-svelte-icons";
    import { compare } from "compare-versions";

    let aws_cli_ver: string;
    let aws_cli_installed: boolean;
    let aws_cli_validity: boolean;
    let open: boolean;
    let counter: number;
    let counter_color: string;
    let spinner = false;
    let running_login_status = false;
    let runningLoginPid: number;
    let unlisten: UnlistenFn;
    let openSuccessToast = false;
    let successProfile: string;
    let timeoutToast = false;
    let timeoutProfile: string;
    let finishedSuccessfully = false;
    let processTimedOut = false;

    function timeout() {
        if (counter < 21) {
            counter_color = "text-red-400";
        }
        if (--counter > 0) return setTimeout(timeout, 1000);
    }

    let toast_auth_code: string;
    let toast_auth_url: string;
    interface auth_payload {
        event_type: string;
        auth_code: string;
        auth_url: string;
        process_pid: number;
    }

    interface profile {
        profile_name: string;
        sso_start_url?: string;
        sso_region?: string;
        sso_account_id?: string;
        sso_role_name?: string;
        region?: string;
    }

    let selectedProf: profile = { profile_name: "" };
    let full_profiles: profile[];

    function set_prof(prof: profile) {
        selectedProf = prof;
    }

    function formatDate(dateString: string) {
        if (dateString === "Never Authenticated") {
            return dateString;
        } else if (dateString !== null) {
            let d = new Date(Date.parse(dateString));
            console.log(d.getTime());
            console.log(new Date().getTime());
            if (d.getTime() >= new Date().getTime()) {
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

    async function get_profile_exp(prof: string): Promise<string> {
        return await invoke("get_profile_exp", { prof });
    }

    async function get_full_profiles() {
        full_profiles = await invoke("get_full_profiles");
        selectedProf = { profile_name: "" };
    }

    function start_event(event: auth_payload) {
        toast_auth_code = event.auth_code;
        toast_auth_url = event.auth_url;
        openbrow(toast_auth_url);
        counter = 60;
        open = true;
        counter_color = "text-indigo-400";
        timeout();
    }
    function timeout_event(prof: profile) {
        finishedSuccessfully = false;
        processTimedOut = true;
        timeoutToast = true;
        timeoutProfile = prof.profile_name;
        open = false;
        counter = 0;
        running_login_status = false;
        runningLoginPid = 0;
    }
    function success_event(prof: profile) {
        finishedSuccessfully = true;
        openSuccessToast = true;
        successProfile = prof.profile_name;
        processTimedOut = false;
        open = false;
        counter = 0;
        running_login_status = false;
        runningLoginPid = 0;
    }
    function wrap_up() {
        finishedSuccessfully = false;
        processTimedOut = false;
        unlisten();
    }
    function set_row_color(profile_name: string, selected: string) {
        if (profile_name === selected) {
            return "blue";
        } else {
            return "default";
        }
    }

    async function start_sso_login(full_prof: profile) {
        spinner = true;
        unlisten = await listen<auth_payload>("sso-login", (event) => {
            if (event.payload.event_type === "start") {
                start_event(event.payload);
            } else if (event.payload.event_type === "success") {
                invoke("update_creds", { full_prof });
                get_full_profiles();
                spinner = false;
                success_event(full_prof);
            } else {
                spinner = false;
                timeout_event(full_prof);
            }
        });
        let prof = full_prof.profile_name;
        runningLoginPid = await invoke("do_sso_login", { prof });
        running_login_status = true;
    }

    async function sso_cancel() {
        if (running_login_status) {
            await invoke("do_sso_cancel", { runningLoginPid });
        }
        spinner = false;
        open = false;
        counter = 0;
        running_login_status = false;
        runningLoginPid = 0;

        unlisten();
    }

    async function check_cli_version() {
        aws_cli_ver = await invoke("get_aws_cli_ver");
        if (aws_cli_ver !== "0.0.0") {
            aws_cli_installed = true;
            if (compare(aws_cli_ver, "2.4.25", ">=")) {
                aws_cli_validity = true;
            }
        }
    }
    $: () => {
        if (unlisten) {
            unlisten();
        }
    };
    $: check_cli_version();
    $: get_full_profiles();
</script>

{#if aws_cli_installed}
    {#if aws_cli_validity}
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
                    <p>
                        Auth Code: <span class="font-bold"
                            >{toast_auth_code}</span
                        >
                    </p>
                    <p>
                        <GradientButton
                            size="xs"
                            color="teal"
                            on:click={() => {
                                openbrow(toast_auth_url);
                            }}>Manually open browser</GradientButton
                        >
                    </p>
                    <p>
                        <span class="{counter_color} font-bold">{counter}</span
                        >s until canceled.
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
            {:else if finishedSuccessfully || processTimedOut}
                {wrap_up()}
            {/if}
            {#if openSuccessToast}
                <Toast
                    color="blue"
                    class="shadow-lg shadow-indigo-500/50"
                    position="top-right"
                    on:close={() => {
                        openSuccessToast = false;
                    }}
                >
                    <FireOutline slot="icon" class="w-5 h-5" />
                    <p>
                        Successfully authenticated to <span class="font-bold"
                            >{successProfile}</span
                        >!
                    </p>
                </Toast>
            {/if}
            {#if timeoutToast}
                <Toast
                    color="red"
                    class="shadow-lg shadow-indigo-500/50"
                    position="top-right"
                    on:close={() => {
                        timeoutToast = false;
                    }}
                >
                    <FireOutline slot="icon" class="w-5 h-5" />
                    <p>
                        Authentication timed out for <span class="font-bold"
                            >{timeoutProfile}</span
                        >
                    </p>
                </Toast>
            {/if}
            <div class="flex gap-4">
                <GradientButton
                    size="sm"
                    color="cyanToBlue"
                    class="font-semibold text-sm"
                    on:click={get_full_profiles}
                    >Reload profiles</GradientButton
                >
                {#if selectedProf.profile_name !== ""}
                    <GradientButton
                        size="sm"
                        class="font-semibold right-align text-sm"
                        color="purpleToBlue"
                        disabled={running_login_status}
                        on:click={() => {
                            openSuccessToast = false;
                            timeoutToast = false;
                            start_sso_login(selectedProf);
                        }}
                    >
                        {#if spinner}
                            <Spinner class="mr-3" size="4" color="white" />
                        {/if}
                        {#if spinner}
                            Authenticating
                        {:else}
                            Authenticate
                        {/if}
                        {selectedProf.profile_name}</GradientButton
                    >
                {:else}
                    <GradientButton
                        size="sm"
                        class="right-align text-sm"
                        color="purpleToBlue"
                        disabled="true"
                        >Select a profile to authenticate</GradientButton
                    >
                {/if}
            </div>
            <br />
            <Table hoverable={true} shadow={true} class="text-xs">
                <TableHead>
                    <TableHeadCell>Profile</TableHeadCell>
                    <TableHeadCell>Auth Status</TableHeadCell>
                    <TableHeadCell>Expiration</TableHeadCell>
                </TableHead>
                <TableBody>
                    {#if full_profiles && full_profiles.length}
                        {#each full_profiles as prof}
                            {#await get_profile_exp(prof.profile_name)}
                                <br />
                            {:then exp}
                                <TableBodyRow
                                    color={set_row_color(
                                        prof.profile_name,
                                        selectedProf.profile_name
                                    )}
                                    on:click={() => {
                                        set_prof(prof);
                                    }}
                                >
                                    <TableBodyCell class="font-bold font-mono"
                                        >{prof.profile_name}</TableBodyCell
                                    >
                                    {@const currDate = formatDate(exp)}
                                    <TableBodyCell>
                                        {#if !["Expired", "Never Authenticated"].includes(currDate)}
                                            <Badge
                                                color="green"
                                                class="!p-1 !font-semibold"
                                            >
                                                Fresh
                                                <CheckCircleSolid
                                                    class="ml-1 h-3 w-3"
                                                />
                                            </Badge>
                                        {:else}
                                            <Badge
                                                color="red"
                                                class="!p-1 !font-semibold"
                                            >
                                                Stale
                                                <CloseCircleSolid
                                                    class="ml-1 h-3 w-3"
                                                />
                                            </Badge>
                                        {/if}
                                    </TableBodyCell>
                                    <TableBodyCell>
                                        <p class="font-bold">{currDate}</p>
                                    </TableBodyCell>
                                </TableBodyRow>
                            {/await}
                        {/each}
                    {/if}
                </TableBody>
            </Table>
            {#if full_profiles === undefined || !full_profiles.length}
                <br />
                <p class="font-bold text-red-500 text-md">
                    Your AWS config is empty, doesn't exist, or has no SSO
                    profiles!
                </p>
            {/if}
        </div>
    {:else}
        <div>
            <h1 class="font-bold text-red-500 text-xl">
                Your AWS CLI is too old (version {aws_cli_ver}). Please install the latest CLI!
            </h1>
        </div>
    {/if}
{:else}
    <div>
        <h1 class="font-bold text-red-500 text-xl">
            You currently do not have the AWS CLI installed (or it is not on your PATH). Please install the AWS CLI, or fix your PATH!
        </h1>
    </div>
{/if}
