export type ButlerSsoSession = {
  session_name: string;
  session_expiration: string | null;
  fresh: boolean;
  profile_names: string[];
};

export type ButerSsoProfile = {
  profile_name: string;
  profile_expiration: string | null;
  fresh: boolean;
};

export type ButlerSsoConfig = {
  sessions: ButlerSsoSession[];
  sso_profiles: ButerSsoProfile[];
  legacy_profiles: ButerSsoProfile[];
};
