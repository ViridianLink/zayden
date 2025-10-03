import Cookies from "js-cookie";

export const baseUrl = "http://145.40.184.89:80";

export async function get<T>(endpoint: string): Promise<T> {
    const authToken = Cookies.get("auth-token");

    if (!authToken) {
        throw new Error("authFail");
    }

    const response = await fetch(`${baseUrl}${endpoint}`, {
        headers: { authorization: `Bearer ${authToken}` },
    });

    return await response.json();
}
