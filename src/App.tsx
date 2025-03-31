import { createSignal } from "solid-js";
import butlogo from "./../app-icon.png";
import ThemeSelect from "./components/ThemeSelect";
import { invoke } from "@tauri-apps/api/core";
import { message } from "@tauri-apps/plugin-dialog";
import { LoginType } from "./types/LoginType";
import "./App.css";
import {
  ButerSsoProfile,
  ButlerSsoConfig,
  ButlerSsoLegacyProfile,
  ButlerSsoSession,
} from "./types/ButlerSsoConfig";
import {
  LegacyProfileTable,
  SsoProfileTable,
  SsoSessionTable,
} from "./components/ConfigTables";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

type SelectedRowData = {
  name: string;
  table: "sessions" | "ssos" | "legacies";
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

  async function refresh_profiles_no_deselect() {
    await invoke("refresh_profiles", {});
    await fetch_config();
  }

  async function authenticate_aws() {
    await invoke("authenticate_aws", {
      loginType: loginType(),
      name: name(),
    });
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
  const handleLegacyProfileSelection = (item: ButlerSsoLegacyProfile) => {
    setSelectedRow({ name: item.profile_name, table: "legacies" });
    setName(item.profile_name);
    setLoginType("LegacyProfile");
  };

  const handleSsoProfileSelection = (item: ButerSsoProfile) => {
    setSelectedRow({ name: item.profile_name, table: "ssos" });
    setName(item.profile_name);
    setLoginType("LegacyProfile");
  };

  // Check if a row is selected
  const isSelected = (
    id: string,
    table: "sessions" | "legacies" | "ssos",
  ): boolean => {
    const current = selectedRow();
    return current !== null && current.name === id && current.table === table;
  };

  window.addEventListener("DOMContentLoaded", () => {
    console.log("DOM fully loaded and parsed. Fetching config...");
    refresh_profiles()
      .then(() => {
        console.log("Config fetched successfully.");
      })
      .catch((error) => {
        console.error("Error fetching config:", error);
        message("Error fetching config: " + error.message);
      });
  });

  const appWebview = getCurrentWebviewWindow();
  appWebview.listen<string>("configs-change", (_) => {
    console.log("Configs changed. Fetching config...");
    refresh_profiles_no_deselect()
      .then(() => {
        console.log("Config fetched successfully.");
      })
      .catch((error) => {
        console.error("Error fetching config:", error);
        message("Error fetching config: " + error.message);
      });
  });

  const authBtnText = (): string => {
    if (loginType()) {
      if (loginType() === "SsoSession") {
        return "Authenticate Session: " + name();
      } else if (loginType() === "LegacyProfile") {
        return "Authenticate Profile: " + name();
      }
    }
    return "Select an identity to authenticate";
  };

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

        <div class="w-3xl mx-auto">
          <div class="sticky flex justify-end items-center pt-4">
            {
              /* <button
              class="btn btn-secondary text-secondary-content w-40 ml-4"
              onClick={refresh_profiles}
            >
              Refresh Config
            </button> */
            }

            <button
              class="btn bg-gradient-to-br from-primary to-secondary text-primary-content disabled:opacity-40 min-w-40 mr-4"
              onClick={authenticate_aws}
              disabled={!selectedRow()}
            >
              {authBtnText()}
            </button>
          </div>

          <div class="flex flex-col p-4 gap-6">
            {SsoSessionTable(
              butlerConfig()?.sessions,
              isSelected,
              handleSessionSelection,
            )}

            {SsoProfileTable(
              butlerConfig()?.sso_profiles,
              isSelected,
              handleSsoProfileSelection,
            )}

            {LegacyProfileTable(
              butlerConfig()?.legacy_profiles,
              isSelected,
              handleLegacyProfileSelection,
            )}
          </div>
        </div>
      </div>
    </main>
  );
}

export default App;
