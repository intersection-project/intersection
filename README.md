<div align="center">

# The Intersection Project - Mass pinging, but targeted 😏

</div>

<div align="center">

# Rest in peace, Intersection.

</div>

Dear Intersection Community,

I really do not like to do this. Killing off your passion projects always hurts. But all great things must come to an end, and Intersection's time has come.

As of today—November 11, 2023—I will be officially archiving the Intersection repository and shutting down the Intersection hosted instance.

I loved working on Intersection and pushing the limits of what Discord could do, however I believe Intersection was a little too experimental to be possible on Discord. It did not see many real-world use cases except for finding users without a certain role, and was extremely complicated for even that purpose. It has also been rumored that Discord may supress notifications in messages that mention over around 10 users, which makes Intersection's entire purpose obsolete without extreme refactors (we'd need to create a role for solely the members we're trying to notify).

Due to a lack of real-world use cases and also a lack of motivation to work on the project, I am officially discontinuing it. If there is anyone qualified who would like to take it over, please reach out. I'd love to see what we can do. I may at some point in the future decide to revive the project—all of it's infrastructure still exists—but for now it seems like a waste of resources.

Intersection, you will be missed.

Thank you all for helping make this possible.  
**LogN**

<div align="center">

# About

</div>

Are you looking to invite the bot? Sadly, as noted above, we no longer host an instance of Intersection. Please try to find someone else who is willing to host the bot.

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

It is suggested to use `docker-compose` to start Intersection. You can easily start the bot in one command:

```
docker compose up
```

If you'd like to use one of our pre-compiled images built from CI, you should follow the comments in
`docker-compose.yml`. You can also enable Watchtower to automatically restart the container with any
new image updates. This is exactly how we deploy in production!

When you push to `main`, `thetayloredman/intersection:latest` is built by the `ci` workflow and pushed
to Docker Hub. Then, Watchtower sees the changes on the production server and updates the container.
