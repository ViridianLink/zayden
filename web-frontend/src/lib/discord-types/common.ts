import Cookies from "js-cookie";
import { baseUrl as backendUrl } from "../backend-types";

export const INVITE_URL =
    "https://discord.com/oauth2/authorize?client_id=787490197943091211&permissions=8&response_type=code&redirect_uri=http%3A%2F%2F127.0.0.1%3A3000%2Fauth%2Fcallback&integration_type=0&scope=identify+bot+guilds+applications.commands";
export const BASE_URL = "https://discord.com/api/v10";
export const IMAGE_URL = "https://cdn.discordapp.com";

export async function get<T>(endpoint: string): Promise<T> {
    const authToken = Cookies.get("auth-token");

    if (!authToken) {
        authFail();
    }

    const response = await fetch(`${BASE_URL}${endpoint}`, {
        headers: { authorization: `Bearer ${authToken}` },
    });

    if (response.status == 401) {
        authFail();
    }

    return await response.json();
}

function authFail(): never {
    console.log("No auth token, redirecting to login");
    window.location.href = `${backendUrl}/login`;
    throw new Error("authFail");
}

export type Snowflake = string;
