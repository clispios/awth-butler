enum AuthStatus {
  Fresh = "Fresh",
  Stale = "Stale",
}

interface TableRow {
  profile_name: string;
  auth_status: AuthStatus;
  expiration: Date;
}
