--
-- Name: temp_voice_mode; Type: TYPE; Schema: public; Owner: -
--

CREATE TYPE public.temp_voice_mode AS ENUM (
    'open',
    'spectator',
    'locked',
    'invisible'
);


SET default_tablespace = '';

SET default_table_access_method = heap;


--
-- Name: bingo; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.bingo (
    id bigint NOT NULL,
    day date NOT NULL,
    spaces text[] NOT NULL
);


--
-- Name: bot_tokens; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.bot_tokens (
    token text NOT NULL,
    name text NOT NULL
);


--
-- Name: family; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.family (
    id bigint NOT NULL,
    partner_ids bigint[] DEFAULT '{}'::bigint[] NOT NULL,
    parent_ids bigint[] DEFAULT '{}'::bigint[] NOT NULL,
    children_ids bigint[] DEFAULT '{}'::bigint[] NOT NULL,
    username character varying NOT NULL,
    blocked_ids bigint[] DEFAULT '{}'::bigint[] NOT NULL
);


--
-- Name: gambling; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.gambling (
    id bigint NOT NULL,
    coins bigint DEFAULT 1000 NOT NULL,
    daily date DEFAULT '1970-01-01'::date NOT NULL,
    stamina integer DEFAULT 1 NOT NULL,
    gift date DEFAULT '1970-01-01'::date NOT NULL,
    gems bigint DEFAULT 0 NOT NULL,
    CONSTRAINT coins_must_be_non_negative CHECK ((coins >= 0))
);


--
-- Name: gambling_effects; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.gambling_effects (
    id integer NOT NULL,
    user_id bigint NOT NULL,
    item_id text NOT NULL,
    expiry timestamp without time zone
);


--
-- Name: gambling_effects_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.gambling_effects_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: gambling_effects_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.gambling_effects_id_seq OWNED BY public.gambling_effects.id;


--
-- Name: gambling_goals; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.gambling_goals (
    id integer NOT NULL,
    user_id bigint NOT NULL,
    goal_id text NOT NULL,
    day date DEFAULT '1970-01-01'::date NOT NULL,
    progress bigint DEFAULT 0 NOT NULL,
    target bigint DEFAULT 1 NOT NULL
);


--
-- Name: gambling_goals_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.gambling_goals_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: gambling_goals_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.gambling_goals_id_seq OWNED BY public.gambling_goals.id;


--
-- Name: gambling_inventory; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.gambling_inventory (
    id integer NOT NULL,
    user_id bigint NOT NULL,
    item_id text NOT NULL,
    quantity bigint DEFAULT 0 NOT NULL
);


--
-- Name: gambling_inventory_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.gambling_inventory_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: gambling_inventory_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.gambling_inventory_id_seq OWNED BY public.gambling_inventory.id;


--
-- Name: gambling_mine; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.gambling_mine (
    id bigint NOT NULL,
    miners bigint DEFAULT 0 NOT NULL,
    mines bigint DEFAULT 0 NOT NULL,
    land bigint DEFAULT 0 NOT NULL,
    countries bigint DEFAULT 0 NOT NULL,
    continents bigint DEFAULT 0 NOT NULL,
    planets bigint DEFAULT 0 NOT NULL,
    solar_systems bigint DEFAULT 0 NOT NULL,
    galaxies bigint DEFAULT 0 NOT NULL,
    universes bigint DEFAULT 0 NOT NULL,
    prestige bigint DEFAULT 0 NOT NULL,
    coal bigint DEFAULT 0 NOT NULL,
    iron bigint DEFAULT 0 NOT NULL,
    gold bigint DEFAULT 0 NOT NULL,
    redstone bigint DEFAULT 0 NOT NULL,
    lapis bigint DEFAULT 0 NOT NULL,
    diamonds bigint DEFAULT 0 NOT NULL,
    emeralds bigint DEFAULT 0 NOT NULL,
    tech bigint DEFAULT 0 NOT NULL,
    utility bigint DEFAULT 0 NOT NULL,
    production bigint DEFAULT 0 NOT NULL,
    mine_activity timestamp without time zone DEFAULT '2025-06-13 22:47:46.151686'::timestamp without time zone NOT NULL
);


--
-- Name: gambling_stats; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.gambling_stats (
    user_id bigint NOT NULL,
    max_cash bigint DEFAULT 0 NOT NULL,
    total_cash bigint DEFAULT 0 NOT NULL,
    gifts_given integer DEFAULT 0 NOT NULL,
    gifts_received integer DEFAULT 0 NOT NULL,
    higher_or_lower_score integer DEFAULT 0 NOT NULL,
    weekly_higher_or_lower_score integer DEFAULT 0 NOT NULL
);


--
-- Name: gold_stars; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.gold_stars (
    id bigint NOT NULL,
    number_of_stars integer DEFAULT 0 NOT NULL,
    given_stars integer DEFAULT 0 NOT NULL,
    received_stars integer DEFAULT 0 NOT NULL,
    last_free_star timestamp without time zone NOT NULL
);


--
-- Name: guilds; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.guilds (
    id bigint NOT NULL,
    temp_voice_category bigint,
    temp_voice_creator_channel bigint,
    thread_id integer DEFAULT 0 NOT NULL,
    support_channel_id bigint,
    support_role_ids bigint[] DEFAULT '{}'::bigint[] NOT NULL,
    faq_channel_id bigint,
    suggestions_channel_id bigint,
    review_channel_id bigint,
    xp_blocked_channels bigint[] DEFAULT '{}'::bigint[] NOT NULL
);


--
-- Name: infractions; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.infractions (
    id integer NOT NULL,
    user_id bigint NOT NULL,
    username character varying(255) NOT NULL,
    guild_id bigint NOT NULL,
    infraction_type character varying(255) NOT NULL,
    moderator_id bigint NOT NULL,
    moderator_username character varying(255) NOT NULL,
    points integer DEFAULT 1 NOT NULL,
    reason character varying(255) NOT NULL,
    created_at timestamp without time zone DEFAULT now() NOT NULL
);


--
-- Name: infractions_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.infractions_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: infractions_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.infractions_id_seq OWNED BY public.infractions.id;


--
-- Name: levels; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.levels (
    id bigint NOT NULL,
    total_xp integer DEFAULT 0 NOT NULL,
    last_xp timestamp without time zone DEFAULT to_timestamp((0)::double precision) NOT NULL,
    xp integer DEFAULT 0 NOT NULL,
    level integer DEFAULT 0 NOT NULL,
    message_count integer DEFAULT 0 NOT NULL
);


--
-- Name: lfg_fireteam; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.lfg_fireteam (
    post bigint NOT NULL,
    user_id bigint NOT NULL,
    alternative boolean DEFAULT false NOT NULL
);


--
-- Name: lfg_guilds; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.lfg_guilds (
    id bigint NOT NULL,
    channel_id bigint NOT NULL,
    role_id bigint,
    scheduled_thread_id bigint
);


--
-- Name: lfg_messages; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.lfg_messages (
    id bigint NOT NULL,
    message bigint NOT NULL,
    channel bigint NOT NULL
);


--
-- Name: lfg_posts; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.lfg_posts (
    id bigint NOT NULL,
    owner bigint NOT NULL,
    activity text NOT NULL,
    start_time timestamp with time zone NOT NULL,
    description text DEFAULT ''::text NOT NULL,
    fireteam_size smallint NOT NULL
);


--
-- Name: lfg_users; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.lfg_users (
    id bigint NOT NULL,
    timezone text NOT NULL
);


--
-- Name: reaction_roles; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.reaction_roles (
    id integer NOT NULL,
    guild_id bigint NOT NULL,
    channel_id bigint NOT NULL,
    message_id bigint NOT NULL,
    role_id bigint NOT NULL,
    emoji character varying(255) NOT NULL
);


--
-- Name: reaction_roles_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.reaction_roles_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: reaction_roles_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.reaction_roles_id_seq OWNED BY public.reaction_roles.id;


--
-- Name: servers; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.servers (
    id bigint NOT NULL,
    support_thread_id integer DEFAULT 0 NOT NULL,
    rules_channel_id bigint,
    general_channel_id bigint,
    spoiler_channel_id bigint,
    support_channel_id bigint,
    suggestions_channel_id bigint,
    support_role_id bigint,
    artist_role_id bigint,
    sleep_role_id bigint
);


--
-- Name: tickets; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.tickets (
    id bigint NOT NULL,
    role_ids bigint[] DEFAULT '{}'::bigint[] NOT NULL
);


--
-- Name: voice_channels; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.voice_channels (
    id bigint NOT NULL,
    owner_id bigint NOT NULL,
    trusted_ids bigint[] DEFAULT '{}'::bigint[] NOT NULL,
    password text,
    persistent boolean DEFAULT false NOT NULL,
    invites bigint[] DEFAULT '{}'::bigint[] NOT NULL,
    mode public.temp_voice_mode DEFAULT 'open'::public.temp_voice_mode NOT NULL
);


--
-- Name: gambling_effects id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gambling_effects ALTER COLUMN id SET DEFAULT nextval('public.gambling_effects_id_seq'::regclass);


--
-- Name: gambling_goals id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gambling_goals ALTER COLUMN id SET DEFAULT nextval('public.gambling_goals_id_seq'::regclass);


--
-- Name: gambling_inventory id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gambling_inventory ALTER COLUMN id SET DEFAULT nextval('public.gambling_inventory_id_seq'::regclass);


--
-- Name: infractions id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.infractions ALTER COLUMN id SET DEFAULT nextval('public.infractions_id_seq'::regclass);


--
-- Name: reaction_roles id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.reaction_roles ALTER COLUMN id SET DEFAULT nextval('public.reaction_roles_id_seq'::regclass);


--
-- Name: bingo bingo_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.bingo
    ADD CONSTRAINT bingo_pkey PRIMARY KEY (id);


--
-- Name: bot_tokens bot_tokens_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.bot_tokens
    ADD CONSTRAINT bot_tokens_pkey PRIMARY KEY (token);


--
-- Name: family family_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.family
    ADD CONSTRAINT family_pkey PRIMARY KEY (id);


--
-- Name: gambling_effects gambling_effects_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gambling_effects
    ADD CONSTRAINT gambling_effects_pkey PRIMARY KEY (id);


--
-- Name: gambling_goals gambling_goals_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gambling_goals
    ADD CONSTRAINT gambling_goals_pkey PRIMARY KEY (id);


--
-- Name: gambling_inventory gambling_inventory_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gambling_inventory
    ADD CONSTRAINT gambling_inventory_pkey PRIMARY KEY (id);


--
-- Name: gambling_mine gambling_mine_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gambling_mine
    ADD CONSTRAINT gambling_mine_pkey PRIMARY KEY (id);


--
-- Name: gambling gambling_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gambling
    ADD CONSTRAINT gambling_pkey PRIMARY KEY (id);


--
-- Name: gambling_stats gambling_stats_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gambling_stats
    ADD CONSTRAINT gambling_stats_pkey PRIMARY KEY (user_id);


--
-- Name: gold_stars gold_stars_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gold_stars
    ADD CONSTRAINT gold_stars_pkey PRIMARY KEY (id);


--
-- Name: guilds guilds_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.guilds
    ADD CONSTRAINT guilds_pkey PRIMARY KEY (id);


--
-- Name: infractions infractions_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.infractions
    ADD CONSTRAINT infractions_pkey PRIMARY KEY (id);


--
-- Name: levels levels_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.levels
    ADD CONSTRAINT levels_pkey PRIMARY KEY (id);


--
-- Name: lfg_fireteam lfg_fireteam_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.lfg_fireteam
    ADD CONSTRAINT lfg_fireteam_pkey PRIMARY KEY (post, user_id);


--
-- Name: lfg_guilds lfg_guilds_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.lfg_guilds
    ADD CONSTRAINT lfg_guilds_pkey PRIMARY KEY (id);


--
-- Name: lfg_messages lfg_messages_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.lfg_messages
    ADD CONSTRAINT lfg_messages_pkey PRIMARY KEY (id);


--
-- Name: lfg_posts lfg_posts_pkey1; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.lfg_posts
    ADD CONSTRAINT lfg_posts_pkey1 PRIMARY KEY (id);


--
-- Name: lfg_users lfg_users_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.lfg_users
    ADD CONSTRAINT lfg_users_pkey PRIMARY KEY (id);


--
-- Name: reaction_roles reaction_roles_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.reaction_roles
    ADD CONSTRAINT reaction_roles_pkey PRIMARY KEY (id);


--
-- Name: servers servers_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.servers
    ADD CONSTRAINT servers_pkey PRIMARY KEY (id);


--
-- Name: tickets tickets_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.tickets
    ADD CONSTRAINT tickets_pkey PRIMARY KEY (id);


--
-- Name: gambling_effects unique_user_item; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gambling_effects
    ADD CONSTRAINT unique_user_item UNIQUE (user_id, item_id);


--
-- Name: gambling_inventory uq_gambling_inventory_user_item; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gambling_inventory
    ADD CONSTRAINT uq_gambling_inventory_user_item UNIQUE (user_id, item_id);


--
-- Name: voice_channels voice_channels_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.voice_channels
    ADD CONSTRAINT voice_channels_pkey PRIMARY KEY (id);


--
-- Name: idx_gambling_gifts_given; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_gambling_gifts_given ON public.gambling_stats USING btree (gifts_given DESC);


--
-- Name: idx_gambling_gifts_received; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_gambling_gifts_received ON public.gambling_stats USING btree (gifts_received DESC);


--
-- Name: idx_gambling_goals_user_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_gambling_goals_user_id ON public.gambling_inventory USING btree (user_id);


--
-- Name: idx_gambling_higher_or_lower_score; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_gambling_higher_or_lower_score ON public.gambling_stats USING btree (higher_or_lower_score DESC);


--
-- Name: idx_gambling_inventory_user_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_gambling_inventory_user_id ON public.gambling_inventory USING btree (user_id);


--
-- Name: idx_gambling_max_cash; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_gambling_max_cash ON public.gambling_stats USING btree (max_cash DESC);


--
-- Name: idx_gambling_total_cash; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_gambling_total_cash ON public.gambling_stats USING btree (total_cash DESC);


--
-- Name: idx_lfg_posts_owner_id; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX idx_lfg_posts_owner_id ON public.lfg_posts USING btree (owner);


--
-- Name: gambling_inventory fk_inventory_user; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gambling_inventory
    ADD CONSTRAINT fk_inventory_user FOREIGN KEY (user_id) REFERENCES public.gambling(id) ON DELETE CASCADE;


--
-- Name: gambling_mine gambling; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.gambling_mine
    ADD CONSTRAINT gambling FOREIGN KEY (id) REFERENCES public.gambling(id);


--
-- Name: lfg_fireteam lfg_fireteam_post_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.lfg_fireteam
    ADD CONSTRAINT lfg_fireteam_post_fkey FOREIGN KEY (post) REFERENCES public.lfg_posts(id) ON DELETE CASCADE;


--
-- Name: lfg_messages lfg_messages_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.lfg_messages
    ADD CONSTRAINT lfg_messages_id_fkey FOREIGN KEY (id) REFERENCES public.lfg_posts(id) ON DELETE CASCADE;

