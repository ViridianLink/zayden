This is a much tighter, more focused scope. By stripping away the administrative and role-based overhead, the bot becomes a pure, lightweight utility for discovery, community interaction, and seamless watch parties.

Here is the refined and expanded feature set based on your exact constraints.

---

### 1. User Management & Access (The Foundation)

_Since there are no Discord roles, no account management, and no bans, this module is strictly about linking and guest provisioning._

- **Secure Account Linking:** `/link [jellyfin_username] [jellyfin_password]`. The bot securely verifies the credentials via the Jellyfin API, maps the Discord ID to the Jellyfin User ID in a local database, and immediately discards the password.
- **Unlinking:** `/unlink` removes the mapping, revoking the bot's ability to pull their watch history or make requests on their behalf.
- **Ephemeral Guest Provisioning (The Watch Party Engine):**
  - When a watch party is created, the bot uses the Jellyfin API to create a temporary user (e.g., `guest_[uuid]`).
  - The bot applies a strict, custom Jellyfin User Policy to this account: **Access restricted exclusively to the specific Movie/Show ID** of the watch party. No access to the rest of the library.
  - The bot DMs the guest a secure, one-time login link (or temporary credentials).
  - _Auto-Cleanup:_ 2 hours after the scheduled party ends, the bot automatically deletes the temporary Jellyfin user via the API to keep the user database clean.

### 2. Media Requests & Discovery (The *arr Integration)

_Members have unlimited requests. Movies auto-accept; shows require manual approval. No quotas._

- **Smart Request Routing:** `/request [title]`
  - Checks if the media is already on the server. If yes, provides a deep link to play it.
  - If it's a **Movie**: Automatically submits the request via Overseerr/Jellyseerr and confirms to the user.
  - If it's a **Show**: Submits the request but flags it for manual approval. The bot replies: _"Requested! Since this is a show, it requires manual approval due to file size. I'll let you know when it's greenlit."_
- **Expanded `/recommend` Engine:**
  - `/recommend trending`: Pulls current global blockbusters from TMDB and checks if they are on your server or available to request.
  - `/recommend mix [Movie A] [Movie B]`: "If you liked the vibe of X and Y..." Uses TMDB's keyword and genre overlap API to find a third movie that shares DNA with both.
  - `/recommend for-me`: Analyzes the linked member's Jellyfin watch history. Finds the most common genres/keywords in their history and recommends highly-rated items _currently on the server_ that they haven't seen.
  - `/recommend hidden-gem`: Finds items on your server with high community ratings (IMDb/TMDB) but low view counts among your specific user base.
  - `/recommend because [Movie]`: "Because you watched X..." Fetches TMDB recommendations based on a specific title.

### 3. Watch Parties & Playback (The Jellyfin Integration)

_Focused on native SyncPlay and seamless guest access._

- **Party Creation & SyncPlay Orchestration:** `/party create [Movie/Show] [Date] [Time]`
  - Creates a Discord scheduled event.
  - When the time comes, the bot uses the Jellyfin SyncPlay API to create a SyncPlay group.
  - The bot acts as the "Host" of the SyncPlay group, ensuring that when the actual host presses play/pause/seek, the API forces the same state on all connected clients (including the ephemeral guest accounts).
- **Guest Onboarding Flow:**
  - Guests click a "Join Party" button on the Discord event/message.
  - The bot DMs them their ephemeral Jellyfin credentials and the direct deep-link to the media.
  - The bot provides a quick 3-step text guide in the DM on how to log in and join the SyncPlay group via the Jellyfin web UI or app.
- **Party Reminders:** The bot pings the Discord channel 15 minutes before start time with the SyncPlay group name and instructions.

### 4. Gamification & Community (Trivia Focus)

_No Discord roles, no ratings. Purely focused on a robust, watch-history-driven trivia game using a local SQLite database for scoring._

- **The Trivia Engine (`/trivia`):**
  - `/trivia me`: Generates a question based _only_ on the user's personal watch history.
  - `/trivia server`: Generates a question based on what the _entire server_ has been watching recently.
- **Trivia Tiers (as requested):**
  - _Deep Dive (Last Movie Watched):_ "In _The Matrix_, what was the exact name of the ship Nebuchadnezzar's hovercraft?" or "Which actor played the character that died at the 45-minute mark?" (Pulls from cast/crew and plot keywords).
  - _Recent Fun (Last 10 Movies):_ "Which of the last 10 movies you watched has the longest runtime?" or "Guess the movie from this emoji summary of your last 10 watches."
  - _Yearly Challenge (All watched this year):_ "Out of all movies watched this year, which director appears most frequently?" or "You've watched 40 hours of media this year. What percentage of that was Sci-Fi?"
- **Interactive Mechanics:**
  - Questions are delivered as Discord Embeds with clickable Buttons (A, B, C, D) for multiple-choice answers.
  - First to click the correct button gets the points.

### 5. Utility & Help

_Since admin and notification commands are removed, we just need basic utility._

- `/about`: Shows bot uptime, total media on the server (pulled from Jellyfin API), and total watch parties hosted.

---

### Technical Considerations for this Specific Scope

1.  **Ephemeral Account Security:** When creating the temporary guest accounts via the Jellyfin API, ensure you are setting the `Policy` object correctly. Specifically, set `EnableAllFolders: false` and explicitly define `EnabledFolders: ["<ID_of_the_movie>"]`. This guarantees they literally cannot see the rest of your library.
2.  **SyncPlay API Limits:** Jellyfin's SyncPlay API is relatively new and can be finicky. The bot will need to poll the SyncPlay state frequently to ensure the ephemeral guests haven't desynced, and may need to send "force sync" commands if a guest's client drops out.
3.  **Database Schema:** Since you are dropping Discord roles and native notifications, your SQLite/Postgres database will be very simple. You really only need three tables:
    - `users` (discord_id, jellyfin_user_id)
    - `watch_parties` (party_id, jellyfin_media_id, start_time, guest_jellyfin_user_ids)
    - `trivia_scores` (discord_id, points, timestamp)
4.  *_Overseerr vs. Direct *arr:*_ Even with the "movies auto / shows manual" rule, using Overseerr/Jellyseerr is still highly recommended. You can configure Overseerr to auto-approve movies and require manual approval for shows, meaning the bot just has to pass the request to Overseerr and let Overseerr handle the routing logic.

Here is the formal addendum to your master plan, written in the same structured style as the original brainstorm. You can append this directly to your project documentation.

---

# Module Addendum: Advanced Discovery, Utility, and External Integrations

This section expands the bot’s capabilities beyond core server management into AI-driven discovery, community safety, external ecosystem bridging, and advanced gamification. These features are designed to increase daily active usage and provide high-value utility without requiring administrative overhead.

### 1. AI-Powered Discovery Engine (Expanding `/recommend`)

_Integrating a lightweight LLM to translate natural language "vibes" into actionable media searches, folding the `vibe-check` concept directly into the core recommendation module._

- **`/recommend vibe [description]`**:
  - **How it works:** The user inputs a natural language mood or highly specific scenario (e.g., _"A 90s sci-fi movie that feels like a rainy Tuesday, with a twist ending and no romance"_).
  - **The LLM Engine:** The bot passes this prompt to a lightweight, fast LLM (e.g., a local Ollama instance running Llama-3-8B, or a low-latency cloud API). The LLM is strictly prompted to output a JSON object containing `genres`, `keywords`, `moods`, and `exclude_keywords` (e.g., `{"genres": ["sci-fi", "thriller"], "keywords": ["twist ending", "cyberpunk"], "exclude": ["romance"]}`).
  - **Execution:** The bot parses this JSON and queries the TMDB API, filtering the results to _only_ show movies currently available on the Jellyfin server.
- **Fallback & Refinement:** If the LLM returns no exact matches, the bot automatically relaxes the constraints (e.g., dropping the "twist ending" keyword) and tries again, ensuring the user always gets at least a few relevant results.

### 2. Community Safety & Comfort (`/content-warnings`)

_A critical utility for watch party hosts to ensure all guests are comfortable with the selected media before committing._

- **`/content-warnings [title]`**:
  - **Data Source:** Integrates with the **DoesTheDogDie.com** API (community-sourced trigger warnings) or scrapes the **IMDb Parents Guide**.
  - **Output:** Generates a clean, spoiler-free Discord Embed. It categorizes warnings by severity (e.g., 🟢 Mild, 🟡 Moderate, 🔴 Severe) for categories like Violence, Gore, Sexual Content, and Profanity.
  - **Watch Party Integration:** When a user creates a `/party`, the bot can optionally auto-generate and pin this embed in the party's voice channel or text thread so guests can review it before joining.

### 3. Time Commitment & Planning (`/binge-calculator`)

_Helps members plan their viewing schedules by providing exact time calculations for series._

- **`/binge-calculator [show]`**:
  - **Calculation Engine:** Pulls the exact runtime of every episode from the TMDB API. It automatically subtracts a configurable estimated intro/outro time (defaulting to 2 minutes per episode) to provide a highly accurate "pure content" runtime.
  - **Output Formats:**
    - _Total Time:_ "Total pure runtime: 41 hours, 12 minutes."
    - _Pacing Scenarios:_ "At 2 episodes a day, this will take you 3 weeks." / "At 1 episode a night, this will take you 6 weeks."
    - _Completion Date:_ "If you start today and watch 3 episodes a day, you will finish on [Date]."

### 4. Advanced Visual & Text Gamification (`/guess`)

_Evolving the trivia concept into a highly interactive, multi-modal guessing game._

- **`/guess [mode: visual | text] [difficulty: easy | medium | hard]`**:
  - **Visual Mode:**
    - _Easy:_ The bot heavily pixelates or blurs the movie poster.
    - _Medium:_ The bot extracts and displays only the 5 dominant hex color palette of the poster.
    - _Hard:_ The bot crops the poster to a tiny, unrecognizable 50x50 pixel square.
  - **Text Mode:**
    - The bot pulls the TMDB plot overview and uses a lightweight NLP script to redact all proper nouns (names, places, specific items), replacing them with `[REDACTED]`.
    - _Example:_ "A `[REDACTED]` must travel to `[REDACTED]` to destroy a `[REDACTED]`..."
  - **Answer Submission:** To prevent chat spam and spoilers, the bot uses **Discord Modals** (pop-up text boxes) for users to submit their guesses. The first correct guess wins the points; incorrect guesses give a hint (e.g., "It's a 90s movie" or "The director is Christopher Nolan").

### 5. Watch Habit Tracking (`/streak`)

_A pure bragging-rights metric tracking consecutive days of media consumption._

- **`/streak [@user]`** _(Defaults to the command issuer if no user is tagged)_:
  - **Data Source:** Queries the Jellyfin `/user_usage_stats` or playback reporting API to find days where the user watched at least 1 minute of media.
  - **Output:** Displays the user's Current Streak, All-Time Longest Streak, and Total Days Watched.
  - **Visuals:** Generates a GitHub-style "contribution heatmap" image (using a library like `calendarheatmap`) showing their watch activity over the last year, embedded directly into the Discord response.
  - **Opt-in Privacy:** Users can toggle their streak visibility. If private, `/streak @user` will only show "This user's streak is private," preventing unwanted tracking.

### 6. External Ecosystem Bridge (`/letterboxd-sync`)

_Connects the private Jellyfin server to the user's public Letterboxd profile to highlight library gaps._

- **`/letterboxd-sync [letterboxd_username]`**:
  - **Data Fetching:** Parses the user's public Letterboxd RSS feed (which contains their watched diary and ratings). No complex OAuth is required.
  - **Cross-Referencing:** Compares the Letterboxd diary against the Jellyfin library metadata (matching by IMDb/TMDB IDs for accuracy).
  - **Output Report:**
    - _Overlap:_ "You have watched 142 films on Letterboxd. 68 of them are currently on our server!"
    - _The "Missing" List:_ Identifies films on their Letterboxd that are _not_ on the server. It filters this list to only show films they rated 3.5 stars or higher, and presents the top 5 as a "Highly Recommended Request List" with direct `/request` buttons.

### 7. Streaming Availability Checker (`/justwatch`)

_Provides immediate alternatives if a requested movie isn't on the server or is pending manual approval._

- **`/justwatch [title]`**:
  - **Use Case:** A user requests a movie. The bot sees it's not on the server and requires manual show approval (or is just missing).
  - **Data Source:** Queries the **TMDB Watch Providers API** (or the JustWatch API) for the user's specific region.
  - **Output:** The bot replies: _"This isn't on our server yet, but it's currently streaming on [Netflix/Hulu/Max] in your region. Here is a deep link to watch it there while you wait for our server to get it!"_ (Includes the official streaming service logos in the embed).

---

### Technical Implementation Notes for the Addendum

1.  **LLM Latency for `/recommend vibe`:** To keep the Discord interaction snappy, do not use a massive cloud LLM. Run a quantized 7B/8B model locally via Ollama, or use a highly optimized, low-latency API (like Groq). The prompt must be strictly constrained to output _only_ JSON to prevent parsing errors.
2.  **Letterboxd RSS Parsing:** Letterboxd's RSS feeds update periodically. If a user logs a movie at 8 PM, it might not hit the RSS feed until 9 PM. The bot should gracefully handle cases where the RSS feed is slightly out of sync with the actual website.
3.  **Image Generation for `/guess` and `/streak`:**
    - For `/guess`, use `sharp` (Node.js) or `Pillow` (Python) to manipulate the poster images on the fly. Cache the blurred/cropped versions locally for 24 hours to save CPU cycles if the same movie is guessed multiple times.
    - For `/streak`, generating the heatmap image on the fly is fine, but ensure the bot has a fallback text-only response if the image generation library fails.
4.  **Jellyfin API Rate Limiting:** Commands like `/streak` and `/letterboxd-sync` require pulling large amounts of user data. Implement a local cache (e.g., Redis or SQLite) that refreshes user watch data once every 12 hours. The bot should query the cache, not the live Jellyfin API, for these heavy commands.
