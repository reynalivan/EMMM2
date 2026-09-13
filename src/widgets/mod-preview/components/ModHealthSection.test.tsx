import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '../../../tests/testing/test-utils';
import ModHealthSection, {
  type ModHealthPanelData,
  type ModViewerExternalReview,
} from './ModHealthSection';

const report: ModHealthPanelData = {
  supportLevel: 'experimental',
  issues: [
    {
      severity: 'error',
      code: 'resource_missing',
      message: 'Resource texture.dds is missing.',
      filePath: 'mod.ini',
      line: 12,
    },
    {
      severity: 'warning',
      code: 'orphan_asset',
      message: 'unused.buf is not referenced.',
      filePath: 'unused.buf',
      line: null,
    },
  ],
  assets: {
    referenced: 2,
    inactiveOnly: 1,
    orphan: 1,
    externalReference: 0,
  },
  assetEntries: [
    {
      category: 'referenced',
      relativePath: 'texture.dds',
      sizeBytes: 128,
      sourceFiles: ['mod.ini'],
    },
  ],
  controls: [
    {
      kind: 'key_toggle',
      name: 'Outfit',
      key: '1',
      back: null,
      values: ['0', '1'],
      defaultValue: '0',
    },
  ],
};

const review: ModViewerExternalReview = {
  changes: [
    { kind: 'modified', category: 'ini', relativePath: 'mod.ini' },
    { kind: 'added', category: 'metadata', relativePath: '.mod_viewer.json' },
  ],
  collectionImpact: '1 collection references missing files.',
};

describe('ModHealthSection', () => {
  it('shows health counts, support level, and detailed issue tabs', () => {
    const onOpenIssueFile = vi.fn();
    render(
      <ModHealthSection
        report={report}
        isLoading={false}
        onRecheck={vi.fn()}
        onOpenIssueFile={onOpenIssueFile}
      />,
    );

    expect(screen.getByRole('heading', { name: 'Mod Health' })).toBeInTheDocument();
    expect(screen.getByText('Experimental')).toBeInTheDocument();
    expect(screen.getByText('1 error')).toBeInTheDocument();
    expect(screen.getByText('1 warning')).toBeInTheDocument();
    expect(screen.queryByText('Referenced: 2')).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Details' }));

    expect(screen.getByRole('dialog')).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: 'Issues' })).toBeInTheDocument();
    expect(screen.getByText('Resource texture.dds is missing.')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Open INI' }));
    expect(onOpenIssueFile).toHaveBeenCalledWith('mod.ini');
    fireEvent.click(screen.getByRole('tab', { name: 'Assets' }));
    expect(screen.getAllByText('Referenced')).not.toHaveLength(0);
    expect(screen.getByText('Inactive only')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('tab', { name: 'Controls' }));
    expect(screen.getByText('Outfit')).toBeInTheDocument();
    expect(screen.getByText('1')).toBeInTheDocument();
    expect(screen.getByText('Default: 0')).toBeInTheDocument();
  });

  it('shows and dismisses a non-attributed external change review', () => {
    const onDismissReview = vi.fn();
    render(
      <ModHealthSection
        report={report}
        isLoading={false}
        onRecheck={vi.fn()}
        externalReview={review}
        onDismissReview={onDismissReview}
      />,
    );

    expect(screen.getByText('Changes detected after Mod Viewer launch')).toBeInTheDocument();
    expect(screen.getByText('INI files')).toBeInTheDocument();
    expect(screen.getByText('Viewer metadata')).toBeInTheDocument();
    expect(screen.getByText('1 collection references missing files.')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Dismiss' }));
    expect(onDismissReview).toHaveBeenCalledOnce();
  });

  it('keeps Recheck available when analysis fails', () => {
    const onRecheck = vi.fn();
    render(
      <ModHealthSection
        report={null}
        isLoading={false}
        errorMessage="folder is unavailable"
        onRecheck={onRecheck}
      />,
    );

    expect(
      screen.getByText('Mod Health could not be checked: folder is unavailable'),
    ).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Recheck' }));
    expect(onRecheck).toHaveBeenCalledOnce();
  });
});
