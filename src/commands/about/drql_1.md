:question: **What is DRQL, and how do I use it?**

DRQL, or the Discord Role Query Language, is a simple set-theory-oriented query language designed for Intersection.

**Need a refresher on set theory?** Read {cmd_about_set_theory}.

Recall that Intersection functions by representing roles as sets of users. With that knowledge, DRQL is a very simple language we can use to represent and process queries.

DRQL has a few underlying "primary" types, and those are:

-   String literals or raw names: `abc` or `"abc"` - these represent the name of a **user** or a **role**. If the name contains non-alpha-numeric characters or spaces, quotes must be used. `everyone` and `here` represent everyone and only online people, respectively.
-   ID literals: `{bot_user_id}` - these represent the ID of a user or role.
-   Direct mentions: <@{bot_user_id}> - you can directly @-mention a user or role instead of an ID literal. This is not recommended as it can result in double-pinging a user, and ID or name literals should be preferred instead. This is only needed in the EXTREMELY rare case that a user and role have the same ID.

Using these types, you can use our set of binary infix operators:

-   `A + B` or `A | B`: **Union**: A ∪ B
-   `A & B`: **Intersection**: A ∩ B
-   `A - B`: **Difference**: A \ B

Again, you might want to read up on set theory to understand these.

DRQL queries are automatically detected in your message. Enclose them in `@{{ ... }}` to tell Intersection to query them! If you need to literally use the text `@{{ ... }}`, put a backslash in: `@\{{ ... }}`
...
