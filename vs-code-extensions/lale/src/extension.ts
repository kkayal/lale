import * as vscode from 'vscode';
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
    TransportKind
} from 'vscode-languageclient/node';

let client: LanguageClient;

export function activate(context: vscode.ExtensionContext) {
    const config = vscode.workspace.getConfiguration('lale');
    const lspPath: string = config.get('lale.lsp.path', 'lale-lsp');

    const serverOptions: ServerOptions = {
        command: lspPath,
        transport: TransportKind.stdio,
        options: {
            env: { ...process.env }
        }
    };

    const clientOptions: LanguageClientOptions = {
        documentSelector: [
            { scheme: 'file', language: 'lale' }
        ],
        synchronize: {
            fileEvents: vscode.workspace.createFileSystemWatcher('**/*.lale')
        }
    };

    client = new LanguageClient(
        'lale-lsp',
        'Lale Language Server',
        serverOptions,
        clientOptions
    );

    client.start();
}

export function deactivate(): Thenable<void> | undefined {
    if (!client) {
        return undefined;
    }
    return client.stop();
}
