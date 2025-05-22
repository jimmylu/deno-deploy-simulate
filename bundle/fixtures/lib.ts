async function execute(name: string): Promise<string> {
    console.log("Executing lib.");
    return `Hello ${name}!`
}

function not_used() {
    console.log("Not used.");
}

export { execute, not_used };
