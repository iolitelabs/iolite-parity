// Copyright 2015-2017 Parity Technologies (UK) Ltd.
// This file is part of Parity.

// Parity is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity.  If not, see <http://www.gnu.org/licenses/>.

import { observer } from 'mobx-react';
import React, { Component, PropTypes } from 'react';
import { FormattedMessage } from 'react-intl';
import { connect } from 'react-redux';

import { Errors } from '~/ui';

import Connection from '../Connection';
import Extension from '../Extension';
import FirstRun from '../FirstRun';
import ParityBar from '../ParityBar';
import Requests from '../Requests';
import Snackbar from '../Snackbar';
import Status from '../Status';
import UpgradeParity from '../UpgradeParity';
import UpgradeStore from '../UpgradeParity/store';
import SyncWarning, { showSyncWarning } from '../SyncWarning';

import Store from './store';
import styles from './application.css';

const inFrame = window.parent !== window && window.parent.frames.length !== 0;
const doShowSyncWarning = showSyncWarning();

@observer
class Application extends Component {
  static contextTypes = {
    api: PropTypes.object.isRequired,
    background: PropTypes.string
  }

  static propTypes = {
    blockNumber: PropTypes.object,
    children: PropTypes.node,
    pending: PropTypes.array
  }

  store = new Store(this.context.api);
  upgradeStore = UpgradeStore.get(this.context.api);

  render () {
    const [root] = (window.location.hash || '').replace('#/', '').split('/');
    const isMinimized = root !== '';

    if (inFrame) {
      return (
        <div className={ styles.error }>
          <FormattedMessage
            id='application.frame.error'
            defaultMessage='ERROR: This application cannot and should not be loaded in an embedded iFrame'
          />
        </div>
      );
    }

    return (
      <div>
        {
          isMinimized
            ? this.renderMinimized()
            : this.renderApp()
        }
        {
          doShowSyncWarning
            ? <SyncWarning />
            : null
        }
        <Connection />
        <Requests />
        <ParityBar dapp={ isMinimized } />
      </div>
    );
  }

  renderApp () {
    const { blockNumber, children } = this.props;

    return (
      <div className={ styles.container }>
        <Extension />
        <FirstRun
          onClose={ this.store.closeFirstrun }
          visible={ this.store.firstrunVisible }
        />
        <Snackbar />
        <UpgradeParity upgradeStore={ this.upgradeStore } />
        <Errors />
        {
          blockNumber
            ? <Status upgradeStore={ this.upgradeStore } />
            : null
        }
        <div className={ styles.content }>
          { children }
        </div>
      </div>
    );
  }

  renderMinimized () {
    const { children } = this.props;

    return (
      <div className={ styles.container }>
        <Errors />
        { children }
      </div>
    );
  }
}

function mapStateToProps (state) {
  const { blockNumber } = state.nodeStatus;
  const { hasAccounts } = state.personal;

  return {
    blockNumber,
    hasAccounts
  };
}

export default connect(
  mapStateToProps,
  null
)(Application);
