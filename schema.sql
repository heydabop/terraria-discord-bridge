--
-- PostgreSQL database dump
--

-- Dumped from database version 12.2 (Ubuntu 12.2-2.pgdg18.04+1)
-- Dumped by pg_dump version 12.2 (Ubuntu 12.2-2.pgdg18.04+1)

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
    message text NOT NULL
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
-- Name: death id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.death ALTER COLUMN id SET DEFAULT nextval('public.death_id_seq'::regclass);


--
-- Name: death death_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.death
    ADD CONSTRAINT death_pkey PRIMARY KEY (id);


--
-- PostgreSQL database dump complete
--

