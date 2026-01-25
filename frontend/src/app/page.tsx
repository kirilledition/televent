'use client';

import {
  List,
  Section,
  Cell,
  Headline,
  Text,
  Button,
} from '@telegram-apps/telegram-ui';
import { useLaunchParams } from '@telegram-apps/sdk-react';

export default function Home() {
  const lp = useLaunchParams();

  return (
    <List>
      <Section
        header="Televent Dashboard"
        footer="Welcome to your personal event manager"
      >
        <Cell
          before={<span style={{ fontSize: '24px' }}>📅</span>}
          subtitle="Manage your calendars and events"
        >
          <Headline weight="1">Televent Mini App</Headline>
        </Cell>
      </Section>

      <Section header="Environment Status">
        <Cell subtitle="Current Platform">
          {String(lp.platform)}
        </Cell>
        <Cell subtitle="Version">
          {String(lp.version)}
        </Cell>
      </Section>

      <Section header="Quick Actions">
        <div style={{ padding: '0 16px 16px' }}>
          <Button size="l" stretched>
            Create Your First Event
          </Button>
        </div>
      </Section>

      <Section footer="Successfully initialized with Next.js + Typeshare">
        <Cell multiline>
          <Text>
            Type safety is ensured between Rust backend and TypeScript frontend using Typeshare.
          </Text>
        </Cell>
      </Section>
    </List>
  );
}
