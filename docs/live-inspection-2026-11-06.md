# Ripple inspection — November 6, 2026

Evidence: the signed-in Helium session, user screenshots, and read-only requests to public pages. No account changes or social actions were submitted by the agent.

| Feature | Observed behavior |
| --- | --- |
| Public | `/`, public composer, 256 UTF-16 code unit limit |
| Following | `/?tlmode=following`, timeline and composer |
| Notifications | `/?tlmode=mentions`, “From your notifications”; account was empty |
| Users | `/users?pageno=N`, thumbnail grid, registration/recently-seen counts, arrow pagination |
| Trending | `/trending` redirects to `/?msg=Soon!` |
| Profile | canonical `/<name>`, `#posts`, `#followers`, `#following` switch local panes |
| Profile actions | `like.php?v=ID` on profile, `/like?v=ID` on public feed; repost, own-post pin/delete links present |
| Profile More | View CSS |
| Memories | `/memories` opened a blank page |
| Account | `/account`, local `#account`, `#editprofile`, `#security`, `#privacy` panes |
| Account fields | Username, disabled email, timezone, language, accent swatches |
| Edit Profile | Separate About You and Personalize save forms; personal details, three labeled links, avatar/background/banner, bio, custom CSS |
| Security | Session table; 2FA marked coming soon |
| Privacy | Blocked-user list; empty on inspected account |
| Quotes | Public markup exposes `/quote`, `qid`, `postcontentsQ`, `qimages[]`; submission untested |

The user demonstrated native sign-in and public posting. Other mutations remain unverified. Browser authentication data was not extracted. The running signed-in native client was preserved.
