const renderResult = (value: string): string => value;

const runBash = (command: string): string => command;

export const render = (command: string): string => {
    const output = runBash(command);
    return renderResult(output);
};
