<div align="center">

# The Intersection Project - Discord mentions, supercharged.

</div>

Intersection is a Discord bot and software suite that empowers Discord's @-mentions with advanced
concepts with their roots in set theory, such as taking the **intersection** of two roles.

The bot is powered by a small query language called DRQL, or the Discord Role Query Language.

You can learn more by inviting Intersection to a server and checking out the `/about` commands.

Intersection is split into a few components:

-   The bot frontend, powered by Serenity and Poise
-   The DRQL backend, powered by Logos and LALRPOP along with a custom interpreter

The bot frontend is located in `src` and most of DRQL is located in `src/drql`.

<div align="center">

## Installation & Setup

</div>

Intersection is designed to run on Linux hosts, and that's our preferred target. It should work on
other operating systems, but we don't test it explicitly.

### Before you continue...

A small note: yes, Intersection is open source software. But, I'd like you to consider the following before you try to set up your own instance of the bot.

Intersection is licensed under the AGPL, which means I have no right to stop you from hosting your own instance of Intersection as long as you comply with the terms of the license. I decided to make this project open source to allow for contributions from the public and to further my experience in OSS.

One thing that I considered before making Intersection open-source was the possibility of people branching off on their own and creating their own projects, which are modified versions of Intersection. I have no ability to stop you, but I'd like you to consider the main goal of open-source first -- community. If you make changes to Intersection, I ask that you _please_ contribute them back to the project and don't start your own bot. It's better for everyone involved.

In addition, Intersection relies on many external services to function, and this number is increasing over time.

Self-hosting Intersection is discouraged.

### 1. Install Prerequisites

Intersection is proudly written in Rust, which means you need the Rust toolchain installed. You can install this from [rustup.rs](https://rustup.rs/).

### 2. Obtain Source Code

You must now obtain a copy of Intersection's source code. You can do this by downloading a zip file from the file explorer above, or if you'd like to be able to receive updates and contribute back to the project, you should install [Git](https://git-scm.com/) and run the `git clone` command followed by the URL to this repository.

### 3. Configuration

Next, you need to set up Intersection's runtime configuration. Copy the `.env.template` file to `.env`. Paste your bot token in that file, where it says `YOUR_TOKEN_HERE`

> **How do I obtain a bot token?**
>
> Tokens are the way Discord bots log in. You can obtain one of these by creating a new application on the [Discord Developer Portal](https://discord.com/developers/applications).

### 4. Starting the Bot

You can now start a development build of Intersection by running `cargo run`.

### Production Deployments

If you're deploying in a production environment, we suggest you use [Docker](https://www.docker.com/) to manage a container for Intersection. This is how Intersection is deployed to allow for compilation on a separate machine and containerization.

You can build a Docker image for Intersection with the command `docker build -t intersection .`. For starting Intersection, I suggest that you use the following command (a .env file must be present in your CWD!):

```bash
docker run \
    -d \
    --restart unless-stopped \
    --init \
    --env-file=.env \
    --name intersection \
    intersection
```

If you'd like to deploy on a separate server, I suggest you use a custom Docker registry to do this. Our registry will be known as `10.0.0.2:5000` and `10.0.1.114:443` as those are what is used internally.

Here's _exactly_ how we deploy Intersection in production:

On the build machine, we build a Docker image, tag it, and push it to our local repo:

```bash
docker build -t intersection .
docker tag intersection 10.0.0.2:5000/intersection
docker push 10.0.0.2:5000/intersection
```

On the host machine, we discard the existing running bot and run a new one based on the image we just built:

```bash
docker pull 10.0.1.114:443/intersection
docker stop intersection
docker rm intersection
docker run \
    -d \
    --restart unless-stopped \
    --init \
    --env-file=env \
    --name intersection \
    10.0.1.114:443/intersection
```

**BIG TODO:** WHEN WE GO OPEN SOURCE, add the repo url to Cargo.toml and update /version to link to the current git ref using build_info::PKG_REPOSITORY
