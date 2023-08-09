:wave: **Hello there, I'm Intersection!** Mass pinging, but targeted �

Intersection is a Discord bot that aims to provide modern and advanced tools for Discord server administration, specifically in the area of @-mentions.

Most Discord users know what a mention is. Informally known as a "ping," they're a method of getting a member (or a group members') attention. This is a mention: <@{bot_user_id}> (oh hey, that's me!)

Discord also has a facility for mentioning roles -- groups of users. However, in the communities I've owned, I've found a need for being able to narrow this down even further.

For example:

-   How can I mention every _online_ admin?
-   How can I mention everyone _without_ a certain role?
-   How can I mention everyone with _two_ specific roles?

Discord does not provide a simple way to do this. In order to make this possible, Intersection takes concepts from a branch of mathematics known as Set Theory and applies them to Discord mentions. This allows you to make more complex queries in a format we call DRQL - the Discord Role Query Language.

Using DRQL, we might describe the above queries as:

-   `@{{ admins & here }}` - the **intersection** of the role `admins` and `here` (everyone who is both an admin and online - this is where Intersection gets its
    name!)
-   `@{{ everyone - "my role" }}` - the **difference** of the role `everyone` and `my role`
-   `@{{ "role 1" & "role 2" }}` - another intersection operation!

...
