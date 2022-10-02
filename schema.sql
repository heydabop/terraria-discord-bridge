--
-- PostgreSQL database dump
--

-- Dumped from database version 12.4 (Debian 12.4-3)
-- Dumped by pg_dump version 12.4 (Debian 12.4-3)

SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SELECT pg_catalog.set_config('search_path', '', false);
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;

SET default_tablespace = '';

SET default_table_access_method = heap;

--
-- Name: death; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.death (
    id bigint NOT NULL,
    create_date timestamp with time zone DEFAULT now() NOT NULL,
    victim character varying(255) NOT NULL,
    killer character varying(255),
    weapon character varying(255),
    message text NOT NULL,
    seconds_since_last integer,
    is_pk boolean DEFAULT false NOT NULL
);


--
-- Name: death_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.death_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: death_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.death_id_seq OWNED BY public.death.id;


--
-- Name: message; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.message (
    id bigint NOT NULL,
    create_date timestamp with time zone DEFAULT now() NOT NULL,
    author character varying(255) NOT NULL,
    content character varying(1024) NOT NULL
);


--
-- Name: message_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.message_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: message_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.message_id_seq OWNED BY public.message.id;


--
-- Name: server_join; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.server_join (
    id bigint NOT NULL,
    create_date timestamp with time zone DEFAULT now() NOT NULL,
    username character varying(255) NOT NULL
);


--
-- Name: server_join_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.server_join_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: server_join_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.server_join_id_seq OWNED BY public.server_join.id;


--
-- Name: server_leave; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.server_leave (
    id bigint NOT NULL,
    create_date timestamp with time zone DEFAULT now() NOT NULL,
    username character varying(255) NOT NULL
);


--
-- Name: server_leave_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.server_leave_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: server_leave_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.server_leave_id_seq OWNED BY public.server_leave.id;


--
-- Name: death id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.death ALTER COLUMN id SET DEFAULT nextval('public.death_id_seq'::regclass);


--
-- Name: message id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.message ALTER COLUMN id SET DEFAULT nextval('public.message_id_seq'::regclass);


--
-- Name: server_join id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.server_join ALTER COLUMN id SET DEFAULT nextval('public.server_join_id_seq'::regclass);


--
-- Name: server_leave id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.server_leave ALTER COLUMN id SET DEFAULT nextval('public.server_leave_id_seq'::regclass);


--
-- Name: death death_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.death
    ADD CONSTRAINT death_pkey PRIMARY KEY (id);


--
-- Name: message message_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.message
    ADD CONSTRAINT message_pkey PRIMARY KEY (id);


--
-- Name: server_join server_join_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.server_join
    ADD CONSTRAINT server_join_pkey PRIMARY KEY (id);


--
-- Name: server_leave server_leave_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.server_leave
    ADD CONSTRAINT server_leave_pkey PRIMARY KEY (id);


--
-- Name: TABLE death; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT ON TABLE public.death TO terraria;


--
-- Name: SEQUENCE death_id_seq; Type: ACL; Schema: public; Owner: -
--

GRANT USAGE ON SEQUENCE public.death_id_seq TO terraria;


--
-- Name: TABLE message; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT ON TABLE public.message TO terraria;


--
-- Name: SEQUENCE message_id_seq; Type: ACL; Schema: public; Owner: -
--

GRANT USAGE ON SEQUENCE public.message_id_seq TO terraria;


--
-- Name: TABLE server_join; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT ON TABLE public.server_join TO terraria;


--
-- Name: SEQUENCE server_join_id_seq; Type: ACL; Schema: public; Owner: -
--

GRANT USAGE ON SEQUENCE public.server_join_id_seq TO terraria;


--
-- Name: TABLE server_leave; Type: ACL; Schema: public; Owner: -
--

GRANT SELECT,INSERT ON TABLE public.server_leave TO terraria;


--
-- Name: SEQUENCE server_leave_id_seq; Type: ACL; Schema: public; Owner: -
--

GRANT USAGE ON SEQUENCE public.server_leave_id_seq TO terraria;


--
-- PostgreSQL database dump complete
--

