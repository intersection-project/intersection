:question: **What is Set Theory, and how does Intersection use it?**

Note: I am not a mathematician, and this is just a basic explanation from my knowledge. If anyone would like to correct information here, please let me know! This is only supposed to be a basic explanation.

Here comes the annoying part. The theory. _Set_ theory. (dunn dunn dunnnnnnn)

Set Theory involves looking at things as sets - groups of things. Let's just imagine a random pile of colored rocks. This isn't a _pile_ of rocks, it's a _set_ of them!

This is math though, so keeping it abstracted like this is hard. Sets are already an abstract concept. Instead, let's think about numbers. Sets of numbers.

Here's two sets:

A: `{{ 1, 2, 3, 7, 9 }}`
B: `{{ 8, 2, 4, 6, 5 }}`

We can now think of a few operations we can perform on these sets. The core 3 set theory operations are:

-   **Union:** The resulting set is all members of _either_ A or B (notated as A ∪ B)
-   **Intersection:** The resulting set is all members of _both_ A and B (notated as A ∩ B)
-   **Difference:** The resulting set is all members of A that are _not_ members of B (many notations, commonly A \ B or A - B)

This is hard to explain with text. Make a quick internet search to find some graphics for these.

Either way, the union, intersection, and difference of our two example sets above is:

-   A ∪ B = `{{ 1, 2, 3, 7, 9, 8, 4, 6, 5 }}`
-   A ∩ B = `{{ 2 }}`
-   A \ B = `{{ 1, 3, 7, 9 }}`
-   B \ A = `{{ 8, 4, 6, 5 }}`

Now that you hopefully understand the union, intersection, and difference operations, this is what Intersection does:

-   Instead of a role being a tag associated with users, a role is a set of users.
-   A user itself can be represented as a set with just that user.
-   These set operations can be applied to roles and users to create advanced mentions.

Now it's time to learn about DRQL's syntax: {cmd_about_drql}
