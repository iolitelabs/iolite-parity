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

import BigNumber from 'bignumber.js';
import React, { Component } from 'react';
import PropTypes from 'prop-types';
import { FormattedMessage } from 'react-intl';
import { connect } from 'react-redux';
import { bindActionCreators } from 'redux';
import { observer } from 'mobx-react';

import * as RequestsActions from '@parity/shared/redux/providers/signerActions';
import Container from '@parity/ui/Container';
import RequestPending from '@parity/ui/Signer/RequestPending';
import Store from '@parity/shared/mobx/signerStore';

import PluginStore from '../pluginStore';

import styles from './embedded.css';

@observer
class Embedded extends Component {
  static contextTypes = {
    api: PropTypes.object.isRequired
  };

  static propTypes = {
    accounts: PropTypes.object.isRequired,
    actions: PropTypes.shape({
      startConfirmRequest: PropTypes.func.isRequired,
      startRejectRequest: PropTypes.func.isRequired
    }).isRequired,
    externalLink: PropTypes.string,
    gasLimit: PropTypes.object.isRequired,
    netVersion: PropTypes.string.isRequired,
    signer: PropTypes.shape({
      finished: PropTypes.array.isRequired,
      pending: PropTypes.array.isRequired
    }).isRequired
  };

  store = new Store(this.context.api, false, this.props.externalLink);
  pluginStore = PluginStore.get();

  render () {
    return (
      <Container style={ { background: 'transparent' } }>
        <div className={ styles.signer }>
          { this.renderPendingRequests() }
        </div>
      </Container>
    );
  }

  renderPendingRequests () {
    const { signer } = this.props;
    const { pending } = signer;

    if (!pending.length) {
      return (
        <div className={ styles.none }>
          <FormattedMessage
            id='signer.embedded.noPending'
            defaultMessage='There are currently no pending requests awaiting your confirmation'
          />
        </div>
      );
    }

    return (
      <div>
        {
          pending
            .sort(this._sortRequests)
            .map(this.renderPending)
        }
      </div>
    );
  }

  renderPending = (data, index) => {
    const { accounts, actions, gasLimit, netVersion } = this.props;
    const { date, id, isSending, payload, origin } = data;
    const transaction = payload.sendTransaction || payload.signTransaction;
    let Handler;

    if (transaction) {
      Handler = this.pluginStore.findHandler(payload, accounts[transaction.from]);
    }

    return (
      <RequestPending
        className={ styles.request }
        date={ date }
        elementRequest={ Handler }
        focus={ index === 0 }
        gasLimit={ gasLimit }
        id={ id }
        isSending={ isSending }
        netVersion={ netVersion }
        key={ id }
        onConfirm={ actions.startConfirmRequest }
        onReject={ actions.startRejectRequest }
        origin={ origin }
        payload={ payload }
        signerStore={ this.store }
      />
    );
  }

  _sortRequests = (a, b) => {
    return new BigNumber(a.id).cmp(b.id);
  }
}

function mapStateToProps (state) {
  const { gasLimit, netVersion } = state.nodeStatus;
  const { accounts } = state.personal;
  const { actions, signer } = state;

  return {
    accounts,
    actions,
    gasLimit,
    netVersion,
    signer
  };
}

function mapDispatchToProps (dispatch) {
  return {
    actions: bindActionCreators(RequestsActions, dispatch)
  };
}

export default connect(
  mapStateToProps,
  mapDispatchToProps
)(Embedded);
