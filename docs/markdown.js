/**
 * 構造化ブロック列を Markdown 文字列に整形する。
 *
 * ページ(スライド/シート)の概念は本文に持ち込まない。
 * ページ区切りは `ExtractionResult.texts` の配列要素として表現される。
 */
export function renderMarkdown(blocks) {
    return blocks
        .map(renderBlock)
        .filter((s) => s.length > 0)
        .join('\n\n');
}
function renderBlock(block) {
    switch (block.type) {
        case 'heading': {
            const level = Math.min(Math.max(block.level, 1), 6);
            return '#'.repeat(level) + ' ' + renderRuns(block.runs);
        }
        case 'para':
            return renderRuns(block.runs);
        case 'list':
            return renderList(block.ordered, block.items);
        case 'table':
            return renderTable(block.rows);
        case 'code':
            return '```\n' + block.text + '\n```';
    }
}
function renderList(ordered, items) {
    // レベルごとに連番を振り直す
    const counters = [];
    return items
        .map((item) => {
        const level = Math.max(0, item.level);
        counters.length = level + 1;
        counters[level] = (counters[level] ?? 0) + 1;
        const marker = ordered ? `${counters[level]}. ` : '- ';
        return '  '.repeat(level) + marker + renderRuns(item.runs);
    })
        .join('\n');
}
function renderTable(rows) {
    if (rows.length === 0)
        return '';
    const cols = Math.max(...rows.map((r) => r.length));
    if (cols === 0)
        return '';
    const line = (cells) => {
        const out = [];
        for (let i = 0; i < cols; i++)
            out.push(escapeCell(renderRuns(cells[i] ?? [])));
        return `| ${out.join(' | ')} |`;
    };
    const header = line(rows[0] ?? []);
    const sep = `| ${new Array(cols).fill('---').join(' | ')} |`;
    const body = rows.slice(1).map(line);
    return [header, sep, ...body].join('\n');
}
/** セル内で表を壊す文字を無害化する */
function escapeCell(s) {
    return s
        .replace(/\|/g, '\\|')
        .replace(/\r?\n/g, '<br>')
        .trim();
}
export function renderRuns(runs) {
    return runs.map(renderRun).join('');
}
/**
 * 1つの Run を装飾付き Markdown に変換する。
 *
 * 記号は前後の空白の外側に置く（`** 太字 **` は描画されないため）。
 */
function renderRun(run) {
    const text = run.text;
    if (text.length === 0)
        return '';
    const lead = text.match(/^\s*/)?.[0] ?? '';
    const trail = text.length > lead.length ? (text.match(/\s*$/)?.[0] ?? '') : '';
    let core = text.slice(lead.length, text.length - trail.length);
    if (core.length === 0)
        return text;
    if (run.code) {
        core = '`' + core + '`';
    }
    else {
        if (run.bold && run.italic)
            core = '***' + core + '***';
        else if (run.bold)
            core = '**' + core + '**';
        else if (run.italic)
            core = '*' + core + '*';
        if (run.strike)
            core = '~~' + core + '~~';
        if (run.highlight)
            core = '==' + core + '==';
    }
    if (run.link)
        core = '[' + core + '](' + run.link + ')';
    return lead + core + trail;
}
//# sourceMappingURL=markdown.js.map