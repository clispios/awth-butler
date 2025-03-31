import { For } from "solid-js";
import {
  ButerSsoProfile,
  ButlerSsoLegacyProfile,
  ButlerSsoSession,
} from "../types/ButlerSsoConfig";
import FreshBadge from "./FreshBadge";
import StaleBadge from "./StaleBadge";
import displayDate from "../utils/DisplayDate";

export function LegacyProfileTable(
  profiles: ButlerSsoLegacyProfile[] | undefined,
  isSelectedFn: (name: string, table: "sessions" | "legacies") => boolean,
  onClickFn: (prof: ButlerSsoLegacyProfile) => void,
) {
  return (
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
            <For each={profiles}>
              {(prof) => (
                <tr
                  class={`hover:bg-base-300 hover:text-base-content cursor-pointer ${
                    isSelectedFn(prof.profile_name, "legacies")
                      ? "bg-secondary text-secondary-content font-semibold"
                      : ""
                  }`}
                  onClick={() => onClickFn(prof)}
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
  );
}

function listToUnorderedList(list: string[]) {
  return (
    <ul>
      <For each={list.sort()}>
        {(item) => <li>{item}</li>}
      </For>
    </ul>
  );
}

export function SsoSessionTable(
  sessions: ButlerSsoSession[] | undefined,
  isSelectedFn: (name: string, table: "sessions" | "legacies") => boolean,
  onClickFn: (prof: ButlerSsoSession) => void,
) {
  return (
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
            <For
              each={sessions?.sort((a, b) =>
                a.session_name.localeCompare(b.session_name)
              )}
            >
              {(sess) => (
                <tr
                  class={`hover:bg-base-300 hover:text-base-content cursor-pointer ${
                    isSelectedFn(sess.session_name, "sessions")
                      ? "bg-secondary text-secondary-content font-semibold"
                      : ""
                  }`}
                  onClick={() => onClickFn(sess)}
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
  );
}

export function SsoProfileTable(
  profiles: ButerSsoProfile[] | undefined,
  isSelectedFn: (
    name: string,
    table: "sessions" | "legacies" | "ssos",
  ) => boolean,
  onClickFn: (prof: ButerSsoProfile) => void,
) {
  return (
    <div class="w-full overflow-x-auto">
      <h3 class="font-bold mb-2">Session Profiles</h3>
      <div class="overflow-x-auto rounded-box border border-base-content/10">
        <table class="table w-full">
          <thead class="bg-base-200">
            <tr>
              <th>Name</th>
              <th>Session</th>
              <th>Status</th>
              <th>Expiration</th>
            </tr>
          </thead>
          <tbody>
            <For
              each={profiles?.sort((a, b) =>
                a.profile_name.localeCompare(b.profile_name)
              )}
            >
              {(prof) => (
                <tr
                  class={`hover:bg-base-300 hover:text-base-content cursor-pointer ${
                    isSelectedFn(prof.profile_name, "ssos")
                      ? "bg-secondary text-secondary-content font-semibold"
                      : ""
                  }`}
                  onClick={() => onClickFn(prof)}
                >
                  <td>{prof.profile_name}</td>
                  <td>{prof.session_name}</td>
                  <td>{prof.fresh ? FreshBadge() : StaleBadge()}</td>
                  <td>{displayDate(prof.profile_expiration)}</td>
                </tr>
              )}
            </For>
          </tbody>
        </table>
      </div>
    </div>
  );
}
