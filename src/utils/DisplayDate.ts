function displayDate(dateString: string | null): string {
  if (dateString === null) {
    return "Never Authenticated";
  } else {
    let d = new Date(Date.parse(dateString));
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
}
export default displayDate;
