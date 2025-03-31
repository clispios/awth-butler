import { createSignal, For } from "solid-js";
import butlogo from "./../app-icon.png";
import ThemeSelect from "./components/ThemeSelect";
import { invoke } from "@tauri-apps/api/core";
import { message } from "@tauri-apps/plugin-dialog";
import { LoginType } from "./types/LoginType";
import "./App.css";
import {
  ButerSsoProfile,
  ButlerSsoConfig,
  ButlerSsoSession,
} from "./types/ButlerSsoConfig";
import FreshBadge from "./components/FreshBadge";
import StaleBadge from "./components/StaleBadge";
import displayDate from "./utils/DisplayDate";

// @ts-ignore TS6133
import clickOutside from "./utils/ClickOutside";

type SelectedRowData = {
  name: string;
  table: "sessions" | "legacies";
};

function App() {
  const [name, setName] = createSignal<string | null>(null);
  const [loginType, setLoginType] = createSignal<LoginType | null>(null);
  const [butlerConfig, setButlerConfig] = createSignal<ButlerSsoConfig | null>(
    null,
  );
  const [selectedRow, setSelectedRow] = createSignal<SelectedRowData | null>(
    null,
  );

  async function fetch_config() {
    let butConf: ButlerSsoConfig = await invoke("fetch_butler_config", {});
    setButlerConfig(butConf);
  }

  async function refresh_profiles() {
    resetSelection();
    await invoke("refresh_profiles", {});
    await fetch_config();
  }

  async function authenticate_aws() {
    await invoke("authenticate_aws", {
      loginType: loginType(),
      name: name(),
    });
    await refresh_profiles();
  }

  const resetSelection = () => {
    setSelectedRow(null);
    setName(null);
    setLoginType(null);
  };

  const handleSessionSelection = (item: ButlerSsoSession) => {
    setSelectedRow({ name: item.session_name, table: "sessions" });
    setName(item.session_name);
    setLoginType("SsoSession");
  };

  // Function to handle row selection in table 2
  const handleLegacyProfileSelection = (item: ButerSsoProfile) => {
    setSelectedRow({ name: item.profile_name, table: "legacies" });
    setName(item.profile_name);
    setLoginType("LegacyProfile");
  };

  // Check if a row is selected
  const isSelected = (id: string, table: "sessions" | "legacies") => {
    const current = selectedRow();
    return current && current.name === id && current.table === table;
  };

  function listToUnorderedList(list: string[]) {
    return (
      <ul>
        <For each={list.sort()}>
          {(item) => <li>{item}</li>}
        </For>
      </ul>
    );
  }

  window.addEventListener("DOMContentLoaded", () => {
    console.log("DOM fully loaded and parsed. Fetching config...");
    fetch_config()
      .then(() => {
        console.log("Config fetched successfully.");
      })
      .catch((error) => {
        console.error("Error fetching config:", error);
        message("Error fetching config: " + error.message);
      });
  });

  return (
    <main>
      <div class="min-h-screen flex flex-col">
        <div class="navbar flex bg-neutral text-neutral-content p-3">
          <img
            src={butlogo}
            class="h-20 pr-3"
            alt="logo"
          />
          <h1 class="text-3xl font-semibold">
            Awth Butler
          </h1>
          {ThemeSelect("float-right ml-auto pr-2 align-middle")}
        </div>
        <div class="w-2xl mx-auto">
          <div class="sticky flex justify-between items-center p-4">
            <button class="btn btn-accent w-40 ml-4" onClick={refresh_profiles}>
              Refresh Config
            </button>
            <button
              class="btn bg-gradient-to-br from-primary to-secondary text-primary-content disabled:opacity-40 min-w-40 mr-4"
              onClick={authenticate_aws}
              disabled={!selectedRow()}
            >
              Authenticate {loginType()
                ? loginType() === "SsoSession" ? "Session:" : "Profile:"
                : ""} {name()}
            </button>
          </div>
          <div class="flex flex-col p-4 gap-6">
            {/* First Table */}
            <div class="w-full overflow-x-auto">
              <h3 class="font-bold mb-2">Sessions</h3>
              <div class="overflow-x-auto rounded-box border border-base-content/10">
                <table class="table w-full">
                  <thead class="bg-base-200">
                    <tr>
                      <th>Name</th>
                      <th>Associated Profiles</th>
                      <th>Status</th>
                      <th>Expiration</th>
                    </tr>
                  </thead>
                  <tbody>
                    <For each={butlerConfig()?.sessions}>
                      {(sess) => (
                        <tr
                          class={`hover:bg-base-300 hover:text-base-content cursor-pointer ${
                            isSelected(sess.session_name, "sessions")
                              ? "bg-info text-info-content"
                              : ""
                          }`}
                          onClick={() => handleSessionSelection(sess)}
                        >
                          <td>{sess.session_name}</td>
                          <td>
                            {listToUnorderedList(sess.profile_names)}
                          </td>
                          <td>{sess.fresh ? FreshBadge() : StaleBadge()}</td>
                          <td>{displayDate(sess.session_expiration)}</td>
                        </tr>
                      )}
                    </For>
                  </tbody>
                </table>
              </div>
            </div>

            {/* Second Table */}
            <div class="w-full overflow-x-auto">
              <h3 class="font-bold mb-2">Legacy Profiles</h3>
              <div class="overflow-x-auto rounded-box border border-base-content/10">
                <table class="table w-full">
                  <thead class="bg-base-200">
                    <tr>
                      <th>Name</th>
                      <th>Status</th>
                      <th>Expiration</th>
                    </tr>
                  </thead>
                  <tbody>
                    <For each={butlerConfig()?.legacy_profiles}>
                      {(prof) => (
                        <tr
                          class={`hover:bg-base-300 hover:text-base-content cursor-pointer ${
                            isSelected(prof.profile_name, "legacies")
                              ? "bg-info text-info-content"
                              : ""
                          }`}
                          onClick={() => handleLegacyProfileSelection(prof)}
                        >
                          <td>{prof.profile_name}</td>
                          <td>{prof.fresh ? FreshBadge() : StaleBadge()}</td>
                          <td>{displayDate(prof.profile_expiration)}</td>
                        </tr>
                      )}
                    </For>
                  </tbody>
                </table>
              </div>
            </div>
          </div>
        </div>
      </div>
    </main>
  );
}

export default App;
