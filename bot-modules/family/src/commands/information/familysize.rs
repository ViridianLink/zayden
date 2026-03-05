/*
    @commands.command(
        aliases=["treesize", "fs", "ts"],
        application_command_meta=commands.ApplicationCommandMeta(
            options=[
                discord.ApplicationCommandOption(
                    name="user",
                    description="The user who you want to check the family size of.",
                    type=discord.ApplicationCommandOptionType.user,
                    required=False,
                ),
            ],
        ),
    )
    @commands.defer()
    @commands.cooldown(1, 3, commands.BucketType.user)
    @vbu.checks.bot_is_ready()
    @commands.guild_only()
    @commands.bot_has_permissions(send_messages=True)
    async def familysize(
        self, ctx: vbu.Context, user: Optional[vbu.converters.UserID] = None
    ):
        """
        Gives you the size of your family tree.
        """

        # Get the user's info
        user_id = user or ctx.author.id
        user_name = await utils.DiscordNameManager.fetch_name_by_id(self.bot, user_id)
        user_info = utils.FamilyTreeMember.get(user_id, utils.get_family_guild_id(ctx))

        # Get size
        size = user_info.family_member_count

        # Output
        output = (
            f"There {'are' if size > 1 else 'is'} {size} {'people' if size > 1 else 'person'} "
            f"in **{utils.escape_markdown(user_name)}**'s family tree."
        )
        if user_id == ctx.author.id:
            output = (
                f"There {'are' if size > 1 else 'is'} {size} "
                f"{'people' if size > 1 else 'person'} in your family tree."
            )
        await vbu.embeddify(
            ctx, output, allowed_mentions=discord.AllowedMentions.none()
        )
*/
